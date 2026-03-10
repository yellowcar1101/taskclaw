import type { Flag, Tag } from './types';

// ── Types ──────────────────────────────────────────────────────────────────────
export interface ParsedTask {
  caption: string;
  flagId?: string;
  tagIds: string[];
  startDate?: string;   // YYYY-MM-DD or YYYY-MM-DDTHH:mm
  dueDate?: string;
  reminderAt?: string;
  starred: boolean;
}

export interface ParsedNode {
  parsed: ParsedTask;
  raw: string;
  children: ParsedNode[];
}

// ── Constants ──────────────────────────────────────────────────────────────────
const MO_S = ['jan','feb','mar','apr','may','jun','jul','aug','sep','oct','nov','dec'];
const MO_L = ['january','february','march','april','may','june','july','august','september','october','november','december'];
const WD_S = ['sun','mon','tue','wed','thu','fri','sat'];
const WD_L = ['sunday','monday','tuesday','wednesday','thursday','friday','saturday'];
const DUR_UNITS = new Set(['d','day','days','w','week','weeks','m','month','months','y','year','years',
                            'h','hr','hrs','hour','hours','min','mins','minute','minutes','mo']);

// ── Small helpers ──────────────────────────────────────────────────────────────
const fmtDate = (d: Date) => d.toISOString().slice(0, 10);
const fmtTime = (h: number, m: number) => `${String(h).padStart(2,'0')}:${String(m).padStart(2,'0')}`;

function isWd(w: string): boolean { const l = w.toLowerCase(); return WD_S.includes(l) || WD_L.includes(l); }
function isMo(w: string): boolean { const l = w.toLowerCase(); return MO_S.includes(l.slice(0,3)) && (MO_S.includes(l) || MO_L.includes(l)); }
function wdIdx(w: string): number { const l = w.toLowerCase(); return WD_S.indexOf(WD_L.indexOf(l) !== -1 ? WD_S[WD_L.indexOf(l)] : l.slice(0,3)); }
function moIdx(w: string): number { return MO_S.indexOf(w.toLowerCase().slice(0,3)); }

function isTimeToken(w: string): boolean {
  return /^\d{1,2}:\d{2}(am?|pm?)?$/i.test(w) || /^\d{1,2}(am?|pm?)$/i.test(w);
}

function isStrongDateTok(w: string): boolean {
  const l = w.toLowerCase();
  if (['today','tomorrow','tom','now'].includes(l)) return true;
  if (isWd(l) || isMo(l)) return true;
  if (DUR_UNITS.has(l)) return true;
  if (isTimeToken(w)) return true;
  if (/^\d{1,2}[\/\-]\d{1,2}(\/\d{2,4})?$/.test(w)) return true;
  if (/^(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\d+$/i.test(w)) return true;
  if (/^\d+(st|nd|rd|th)$/i.test(w)) return true;
  if (['am','pm','a','p'].includes(l)) return true;
  return false;
}

// ── Single-token date parser (legacy token syntax: d:DATE, +3d, etc.) ─────────
export function parseDate(token: string): string | undefined {
  const t = token.toLowerCase().trim();
  const today = new Date(); today.setHours(0,0,0,0);

  if (t === 'today' || t === 'tod') return fmtDate(today);
  if (t === 'tomorrow' || t === 'tom') {
    const d = new Date(today); d.setDate(d.getDate() + 1); return fmtDate(d);
  }
  if (t === 'nextweek' || t === 'nw' || t === 'next-week') {
    const d = new Date(today);
    d.setDate(d.getDate() + ((8 - d.getDay()) % 7 || 7));
    return fmtDate(d);
  }
  // Weekday
  const wdi = wdIdx(t);
  if (wdi !== -1) {
    const d = new Date(today); d.setDate(d.getDate() + ((wdi - d.getDay() + 7) % 7 || 7)); return fmtDate(d);
  }
  // Relative: +Nd / +Nw / +Nm
  const rel = t.match(/^\+(\d+)([dwm])$/);
  if (rel) {
    const n = parseInt(rel[1]); const d = new Date(today);
    if (rel[2]==='d') d.setDate(d.getDate()+n);
    else if (rel[2]==='w') d.setDate(d.getDate()+n*7);
    else d.setMonth(d.getMonth()+n);
    return fmtDate(d);
  }
  // ISO
  if (/^\d{4}-\d{2}-\d{2}$/.test(t)) return t;
  // DD/MM/YY(YY)
  const dmy = t.match(/^(\d{1,2})[\/\-](\d{1,2})[\/\-](\d{2,4})$/);
  if (dmy) {
    const yr = dmy[3].length===2 ? '20'+dmy[3] : dmy[3];
    const iso = `${yr}-${dmy[1].padStart(2,'0')}-${dmy[2].padStart(2,'0')}`;
    if (!isNaN(new Date(iso).getTime())) return iso;
  }
  return undefined;
}

// ── Multi-word natural language date+time parser ───────────────────────────────
interface DTResult { date?: string; time?: string; }

function parseDateTimeExpr(words: string[]): DTResult {
  const today = new Date(); today.setHours(0,0,0,0);
  let i = 0;
  let date: Date | undefined;
  let timeH: number | undefined, timeM = 0;

  const peek  = () => (i < words.length ? words[i].toLowerCase() : '');
  const peekR = () => (i < words.length ? words[i] : '');
  const adv   = () => (i < words.length ? words[i++].toLowerCase() : '');
  const advR  = () => (i < words.length ? words[i++] : '');

  function tryTime(): boolean {
    if (i >= words.length) return false;
    const w = peekR(), wl = w.toLowerCase();
    if (wl === 'at') { i++; return tryTime(); }
    // HH:mm[am/pm]
    const hm = w.match(/^(\d{1,2}):(\d{2})(am?|pm?)?$/i);
    if (hm) {
      i++; timeH = parseInt(hm[1]); timeM = parseInt(hm[2]);
      const ap = (hm[3]||'').toLowerCase();
      if ((ap==='pm'||ap==='p') && timeH<12) timeH+=12;
      if ((ap==='am'||ap==='a') && timeH===12) timeH=0;
      return true;
    }
    // N am/pm (two separate tokens)
    if (/^\d+$/.test(w) && i+1 < words.length && ['am','pm','a','p'].includes(words[i+1].toLowerCase())) {
      i++; timeH = parseInt(w); const ap = adv();
      if ((ap==='pm'||ap==='p') && timeH<12) timeH+=12;
      if ((ap==='am'||ap==='a') && timeH===12) timeH=0;
      return true;
    }
    // Npm / Nam
    const hp = w.match(/^(\d{1,2})(am?|pm?)$/i);
    if (hp) {
      i++; timeH = parseInt(hp[1]); timeM = 0;
      const ap = hp[2].toLowerCase();
      if ((ap==='pm'||ap==='p') && timeH<12) timeH+=12;
      if ((ap==='am'||ap==='a') && timeH===12) timeH=0;
      return true;
    }
    return false;
  }

  function parseDuration(): { days: number; mins: number } | null {
    // Parses "N unit [N unit ...]" sequences; returns null if nothing found
    let days=0, mins=0, any=false;
    while (i < words.length && /^\d+$/.test(peek())) {
      const n = parseInt(adv());
      const unit = peek().replace(/s$/,'').toLowerCase();
      if (!DUR_UNITS.has(unit) && !DUR_UNITS.has(unit+'s')) { i--; break; }
      i++; any = true;
      if (['min','minute','mo'].includes(unit)) mins += n;
      else if (['h','hr','hour'].includes(unit)) mins += n*60;
      else if (['d','day'].includes(unit)) days += n;
      else if (['w','week'].includes(unit)) days += n*7;
      else if (['m','month'].includes(unit)) {
        const tmp = new Date(today); tmp.setMonth(tmp.getMonth()+n);
        days += Math.round((tmp.getTime()-today.getTime())/86400000);
      } else if (['y','year'].includes(unit)) days += n*365;
      if (peek() === 'and') i++;
    }
    return any ? { days, mins } : null;
  }

  const w = peek();

  // "in N unit [N unit] ..."
  if (w === 'in') {
    i++;
    const dur = parseDuration();
    if (dur) {
      if (dur.mins > 0 && dur.days === 0) {
        // purely a time offset: "in 30 min"
        const now = new Date();
        now.setMinutes(now.getMinutes() + dur.mins);
        date = new Date(now); date.setHours(0,0,0,0);
        timeH = now.getHours(); timeM = now.getMinutes();
      } else {
        date = new Date(today); date.setDate(date.getDate() + dur.days);
        if (dur.mins > 0) { timeH = Math.floor(dur.mins/60); timeM = dur.mins%60; }
      }
    } else {
      i--; // backtrack — "in" wasn't a date token
    }
  }
  else if (w === 'today' || w === 'now')     { i++; date = new Date(today); }
  else if (w === 'tomorrow' || w === 'tom')  { i++; date = new Date(today); date.setDate(date.getDate()+1); }
  else if (w === 'next') {
    i++;
    const nxt = peek();
    if (isWd(nxt)) {
      const wd = wdIdx(nxt); i++;
      const d = new Date(today);
      d.setDate(d.getDate() + ((wd-d.getDay()+7)%7||7) + 7);
      date = d;
    } else if (nxt === 'week')  { i++; const d = new Date(today); d.setDate(d.getDate()+(8-d.getDay())%7||7+7); date = d; }
    else if (nxt === 'month') { i++; const d = new Date(today); d.setMonth(d.getMonth()+1,1); date = d; }
    else if (nxt === 'year')  { i++; const d = new Date(today); d.setFullYear(d.getFullYear()+1); date = d; }
  }
  else if (isWd(w)) {
    const wd = wdIdx(w); i++;
    const d = new Date(today);
    d.setDate(d.getDate() + ((wd-d.getDay()+7)%7||7));
    date = d;
  }
  else if (isMo(w)) {
    const mi = moIdx(w); i++;
    const dayW = peekR();
    if (/^\d+(st|nd|rd|th)?$/i.test(dayW)) {
      const day = parseInt(advR());
      const d = new Date(today); d.setMonth(mi, day);
      if (d < today) d.setFullYear(d.getFullYear()+1);
      // optional year
      if (/^\d{2,4}$/.test(peek())) { let yr = parseInt(adv()); if (yr<100) yr+=2000; d.setFullYear(yr); }
      date = d;
    }
  }
  else {
    // "Jan26" combined token
    const mdd = peekR().match(/^(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*(\d+)$/i);
    if (mdd) {
      i++; const mi = moIdx(mdd[1]); const day = parseInt(mdd[2]);
      const d = new Date(today); d.setMonth(mi, day);
      if (d < today) d.setFullYear(d.getFullYear()+1);
      date = d;
    }
    // MM/DD or DD-MM or MM/DD/YYYY
    else if (/^\d{1,2}[\/\-]\d{1,2}(\/\d{2,4})?$/.test(peekR())) {
      const raw2 = advR();
      const pts = raw2.split(/[\/\-]/).map(Number);
      const d = new Date(today);
      if (pts.length === 3) {
        let yr = pts[2]; if (yr<100) yr+=2000;
        d.setFullYear(yr, pts[0]-1, pts[1]);
      } else {
        d.setMonth(pts[0]-1, pts[1]);
        if (d < today) d.setFullYear(d.getFullYear()+1);
      }
      date = d;
    }
    else {
      // try pure time
      tryTime();
      if (timeH !== undefined) date = new Date(today);
    }
  }

  // After base date: handle "today in 1h 25min" style time offset
  if (date && timeH === undefined && peek() === 'in') {
    const saved = i; i++;
    const dur = parseDuration();
    if (dur && dur.mins > 0 && dur.days === 0) {
      const now = new Date(); timeH = now.getHours() + Math.floor(dur.mins/60); timeM = now.getMinutes() + dur.mins%60;
      if (timeM >= 60) { timeH++; timeM -= 60; }
    } else { i = saved; }
  }

  // After base date: weekday refinement for "in 3 weeks Fri"
  if (date && timeH === undefined && isWd(peek())) {
    const wd = wdIdx(peek()); i++;
    const diff = (wd - date.getDay() + 7) % 7;
    date.setDate(date.getDate() + diff);
  }

  // Try time
  if (timeH === undefined) tryTime();

  if (!date && timeH === undefined) return {};
  return {
    date: date ? fmtDate(date) : undefined,
    time: timeH !== undefined ? fmtTime(timeH, timeM) : undefined,
  };
}

// ── Reminder expression parser ─────────────────────────────────────────────────
// words = everything after "remind" / "rmd" keyword
function parseReminderWords(
  words: string[],
  dueDate?: string,
  dueTime?: string
): string | undefined {
  let idx = 0;
  // skip "me"
  if ((words[idx]||'').toLowerCase() === 'me') idx++;

  const rest = words.slice(idx);
  if (rest.length === 0) {
    // "remind me" → same as due
    if (dueDate) return dueTime ? `${dueDate}T${dueTime}` : dueDate;
    return undefined;
  }

  // Detect "in advance" / "before" / "ahead" → relative to due
  const restL = rest.map(w => w.toLowerCase());
  const isRelative = restL.includes('advance') || restL.at(-1) === 'before' || restL.at(-1) === 'ahead';

  if (isRelative) {
    // Strip "in advance" / "before" / "ahead" and parse the duration
    const durWords = rest.filter(w => !['in','advance','before','ahead'].includes(w.toLowerCase()));
    let totalMins = 0;
    for (let j = 0; j < durWords.length; j++) {
      const n = parseInt(durWords[j]);
      if (isNaN(n)) continue;
      const unit = (durWords[j+1]||'').toLowerCase().replace(/s$/,'');
      if (['min','minute','mo'].includes(unit)) { totalMins += n; j++; }
      else if (['h','hr','hour'].includes(unit)) { totalMins += n*60; j++; }
      else if (['d','day'].includes(unit)) { totalMins += n*1440; j++; }
    }
    if (totalMins > 0 && dueDate) {
      const base = dueTime ? new Date(`${dueDate}T${dueTime}`) : new Date(`${dueDate}T09:00`);
      base.setMinutes(base.getMinutes() - totalMins);
      return `${fmtDate(base)}T${fmtTime(base.getHours(), base.getMinutes())}`;
    }
    return undefined;
  }

  // Parse as absolute date/time expression
  const dt = parseDateTimeExpr(rest);
  if (dt.date) return dt.time ? `${dt.date}T${dt.time}` : dt.date;
  return undefined;
}

// ── Right-to-left date boundary finder ────────────────────────────────────────
function findDateBoundary(words: string[]): number {
  let dateStart = words.length; // default: no date
  let inDate = false;

  for (let i = words.length - 1; i >= 0; i--) {
    const w = words[i], wl = w.toLowerCase();
    const rW = i + 1 < words.length ? words[i+1].toLowerCase() : '';
    const lW = i > 0 ? words[i-1].toLowerCase() : '';

    if (isStrongDateTok(w)) { dateStart = i; inDate = true; continue; }

    // Bare number: in date context, or neighboured by date tokens
    if (/^\d+$/.test(w)) {
      if (inDate || isStrongDateTok(rW) || isMo(lW) || DUR_UNITS.has(rW)) {
        dateStart = i; inDate = true; continue;
      }
    }
    // "in" — only if right neighbour is a number or date-unit
    if (wl === 'in' && (/^\d+$/.test(rW) || DUR_UNITS.has(rW) || isStrongDateTok(rW))) {
      dateStart = i; inDate = true; continue;
    }
    // "next" — only if right neighbour is weekday/month/week/month/year
    if (wl === 'next' && (isWd(rW) || isMo(rW) || ['week','month','year'].includes(rW))) {
      dateStart = i; inDate = true; continue;
    }
    // "at" — only if right is a time or number
    if (wl === 'at' && (isTimeToken(rW) || /^\d+$/.test(rW))) {
      dateStart = i; inDate = true; continue;
    }
    // "and" bridging two duration chunks ("in 1 month and 2 weeks")
    if (wl === 'and' && inDate) { continue; }

    break; // non-date word — stop scanning
  }
  return dateStart;
}

// ── Master line parser ─────────────────────────────────────────────────────────
// Handles both legacy token syntax AND MLO natural language:
//   Legacy:    "Buy groceries !urgent d:tomorrow #shopping *"
//   NLP:       "Call Jim tomorrow 4pm remind 10 min in advance"
//   Quoted:    "Call Jim" tomorrow remind me
//   Switches:  "Buy umbrella -flGreen -star -s"
//   Combined:  "Send report in 3 days -d remind tomorrow 10am @work"
export function parseTaskLine(raw: string, flags: Flag[], tags: Tag[]): ParsedTask {
  let text = raw.trim();

  // 1. Quoted caption extraction
  let quotedCaption: string | undefined;
  if (text.startsWith('"')) {
    const eq = text.indexOf('"', 1);
    if (eq !== -1) { quotedCaption = text.slice(1, eq); text = text.slice(eq + 1).trim(); }
  }

  const words = text.split(/\s+/).filter(Boolean);
  const remaining: string[] = [];

  let flagId:     string | undefined;
  const tagIds:   string[] = [];
  let startDate:  string | undefined;
  let dueDate:    string | undefined;
  let reminderAt: string | undefined;
  let starred =   false;
  let dateTarget: 's' | 'd' | null = null; // -s or -d switch

  // 2. Pass 1 — extract explicit tokens and switches
  let remindIdx = -1; // index in `remaining` where remind block starts

  for (let i = 0; i < words.length; i++) {
    const w = words[i], wl = w.toLowerCase();

    // Starred
    if (w === '*' || wl === '-star' || wl === '-*') { starred = true; continue; }

    // Flag switches
    if (w.startsWith('!') && w.length > 1) {
      const name = w.slice(1).toLowerCase();
      const f = flags.find(f => f.name.toLowerCase().startsWith(name));
      if (f) { flagId = f.id; continue; }
    }
    const flm = w.match(/^-fl(.+)$/i);
    if (flm) {
      const name = flm[1].toLowerCase();
      const f = flags.find(f => f.name.toLowerCase().startsWith(name));
      if (f) { flagId = f.id; continue; }
    }

    // Tag switches (#tag or @tag)
    if ((w.startsWith('#') || w.startsWith('@')) && w.length > 1) {
      const name = w.slice(1).replace(/[;,]+$/, '').toLowerCase();
      const t = tags.find(t => t.name.toLowerCase() === name);
      if (t) { tagIds.push(t.id); continue; }
      // @word without semicolons: still try after stripping trailing semicolons
    }

    // Explicit date tokens: d:, s:, r:
    const dm = w.match(/^(s|start|d|due|r|rem|reminder):(.+)$/i);
    if (dm) {
      const parsed = parseDate(dm[2]);
      if (parsed !== undefined) {
        const p = dm[1].toLowerCase();
        if (p==='s'||p==='start') startDate = parsed;
        else if (p==='d'||p==='due') dueDate = parsed;
        else reminderAt = parsed;
        continue;
      }
    }

    // -s / -start / -d / -due switches (date field selector)
    if (['-s','-start'].includes(wl)) { dateTarget = 's'; continue; }
    if (['-d','-due'].includes(wl))   { dateTarget = 'd'; continue; }

    // Remind keyword — mark position in remaining and stop collecting remind block
    if (['remind','reminder','rmd'].includes(wl)) { remindIdx = remaining.length; continue; }

    remaining.push(w);
  }

  // 3. Extract remind block from remaining words
  //    Everything from remindIdx onward (in remaining) is the reminder expression
  let remindWords: string[] = [];
  let captionCandidates: string[];
  if (remindIdx !== -1) {
    remindWords = remaining.slice(remindIdx);
    captionCandidates = remaining.slice(0, remindIdx);
  } else {
    captionCandidates = [...remaining];
  }

  // 4. Find inline date in captionCandidates (if no explicit d:/s: already found)
  const needsDate = !startDate && !dueDate;
  let inlineDate: string | undefined;
  let inlineTime: string | undefined;

  if (needsDate && captionCandidates.length > 0) {
    const boundary = findDateBoundary(captionCandidates);
    if (boundary < captionCandidates.length) {
      const dateWords = captionCandidates.slice(boundary);
      const dt = parseDateTimeExpr(dateWords);
      if (dt.date || dt.time) {
        inlineDate = dt.date;
        inlineTime = dt.time;
        captionCandidates = captionCandidates.slice(0, boundary);
      }
    }
  }

  // 5. Apply inline date to the right field
  if (inlineDate || inlineTime) {
    const full = inlineTime ? `${inlineDate}T${inlineTime}` : inlineDate;
    if (dateTarget === 's') startDate = full;
    else dueDate = full; // default: due
  }

  // 6. Parse reminder expression (now we know dueDate/dueTime)
  if (remindIdx !== -1 && !reminderAt) {
    const dueDatePart = (dueDate || startDate || '').split('T')[0];
    const dueTimePart = (dueDate || startDate || '').includes('T') ? (dueDate || startDate || '').split('T')[1] : undefined;
    reminderAt = parseReminderWords(remindWords, dueDatePart || undefined, dueTimePart);
  }

  // 7. Build caption
  const caption = quotedCaption ?? captionCandidates.join(' ');

  return { caption, flagId, tagIds, startDate, dueDate, reminderAt, starred };
}

// ── Indent counter ────────────────────────────────────────────────────────────
function lineIndent(line: string): number {
  let count = 0;
  for (const ch of line) { if (ch==='\t') count+=4; else if (ch===' ') count++; else break; }
  return count;
}

// ── Rapid input parser ────────────────────────────────────────────────────────
export function parseRapidInput(
  text: string, flags: Flag[], tags: Tag[], applyParsing: boolean
): ParsedNode[] {
  const rawLines = text.split('\n');
  const lines = rawLines
    .map(l => ({ indent: lineIndent(l), trimmed: l.trim() }))
    .filter(l => l.trimmed.length > 0);
  if (!lines.length) return [];

  const uniqueIndents = [...new Set(lines.map(l => l.indent))].sort((a,b) => a-b);
  const toLevel = new Map(uniqueIndents.map((v,i) => [v,i]));

  const roots: ParsedNode[] = [];
  const stack: Array<{ node: ParsedNode; level: number }> = [];

  for (const { trimmed, indent } of lines) {
    const level = toLevel.get(indent) ?? 0;
    const parsed = applyParsing
      ? parseTaskLine(trimmed, flags, tags)
      : { caption: trimmed, tagIds: [], starred: false };
    const node: ParsedNode = { parsed, raw: trimmed, children: [] };

    while (stack.length > 0 && stack[stack.length-1].level >= level) stack.pop();
    if (stack.length === 0) roots.push(node); else stack[stack.length-1].node.children.push(node);
    stack.push({ node, level });
  }
  return roots;
}

// ── Flatten for display ───────────────────────────────────────────────────────
export function flattenNodes(nodes: ParsedNode[], depth = 0): Array<{ node: ParsedNode; depth: number }> {
  const result: Array<{ node: ParsedNode; depth: number }> = [];
  for (const node of nodes) {
    result.push({ node, depth });
    if (node.children.length > 0) result.push(...flattenNodes(node.children, depth+1));
  }
  return result;
}
