/**
 * TaskClaw inline parser — §14
 * Parses caption strings with embedded tokens into structured task fields.
 */

import type { Flag, Tag } from './types';

export interface ParsedTask {
  caption: string;
  flagId: string | null;
  tagIds: string[];
  starred: boolean;
  startDate: string | null;   // YYYY-MM-DD or YYYY-MM-DDTHH:mm
  dueDate: string | null;
  reminderAt: string | null;
  importance?: number;
  urgency?: number;
  effort?: number;
  timeRequired?: number;      // minutes
  timeRequiredMax?: number;   // minutes
  leadTime?: number;          // minutes
  colorHex?: string;
  hideInViews?: boolean;
  subtasksInOrder?: boolean;
  isProject?: boolean;
  isFolder?: boolean;
}

// ── Public API ────────────────────────────────────────────────────────────────

export function parseCaption(
  raw: string,
  flags: Flag[],
  tags: Tag[],
): ParsedTask {
  let s = raw;
  let flagId: string | null = null;
  const tagIds: string[] = [];
  let starred = false;
  let startDate: string | null = null;
  let dueDate: string | null = null;
  let reminderAt: string | null = null;
  let importance: number | undefined;
  let urgency: number | undefined;
  let effort: number | undefined;
  let timeRequired: number | undefined;
  let timeRequiredMax: number | undefined;
  let leadTime: number | undefined;
  let colorHex: string | undefined;
  let hideInViews: boolean | undefined;
  let subtasksInOrder: boolean | undefined;
  let isProject: boolean | undefined;
  let isFolder: boolean | undefined;

  // 1. Quoted caption — protect from parsing
  let quotedCaption: string | null = null;
  s = s.replace(/^"([^"]+)"/, (_, q) => { quotedCaption = q; return ''; }).trim();

  // 2. Extract tokens

  // !FlagName
  s = s.replace(/!(\S+)/g, (_, name) => {
    const flag = findFlag(name, flags);
    if (flag) flagId = flag.id;
    return '';
  });

  // -fl=<FlagName> or -fl<FlagName>
  s = s.replace(/-fl(?:=)?(\S+)/gi, (_, name) => {
    const flag = findFlag(name, flags);
    if (flag) flagId = flag.id;
    return '';
  });

  // #tag or @tag (single word after #/@)
  s = s.replace(/[#@](\w+)/g, (_, name) => {
    findOrNoteTag(name, tags, tagIds);
    return '';
  });

  // context <name1>; <name2>; ... — semicolon-separated context names
  s = s.replace(/\bcontext\s+([^-][^\n]*)/gi, (_, rest) => {
    rest.split(/\s*;\s*/).forEach((name: string) => {
      const trimmed = name.trim();
      if (trimmed) findOrNoteTag(trimmed, tags, tagIds);
    });
    return '';
  });

  // @name1; @name2  or  @ name1; name2  — multi-context semicolon-separated block
  // Match an @ followed by a name, then any number of ; (optional @) name continuations
  s = s.replace(/@\s*[\w][^;@\n]*(?:;\s*@?\s*[\w][^;@\n]*)*/g, (full) => {
    full.split(';').forEach(part => {
      const trimmed = part.replace(/^@\s*/, '').trim();
      if (trimmed) findOrNoteTag(trimmed, tags, tagIds);
    });
    return '';
  });

  // * standalone star
  s = s.replace(/\b\*\b/g, () => { starred = true; return ''; });
  // -star or -*
  s = s.replace(/-star\b|-\*/gi, () => { starred = true; return ''; });

  // -i1 to -i5 → importance
  s = s.replace(/-i([1-5])\b/gi, (_, n) => { importance = parseInt(n); return ''; });

  // -u1 to -u5 → urgency
  s = s.replace(/-u([1-5])\b/gi, (_, n) => { urgency = parseInt(n); return ''; });

  // -e1 to -e5 → effort
  s = s.replace(/-e([1-5])\b/gi, (_, n) => { effort = parseInt(n); return ''; });

  // -tmax<time> → time_required_max (must come before -t<time>)
  s = s.replace(/-tmax(\S+)/gi, (_, t) => { timeRequiredMax = parseTimeExpr(t); return ''; });

  // -t<time> → time_required
  s = s.replace(/-t(\S+)/gi, (_, t) => { timeRequired = parseTimeExpr(t); return ''; });

  // -l<time> → lead_time
  s = s.replace(/-l(\S+)/gi, (_, t) => { leadTime = parseTimeExpr(t); return ''; });

  // -p → is_project
  s = s.replace(/-p\b/gi, () => { isProject = true; return ''; });

  // -f → is_folder
  s = s.replace(/-f\b/gi, () => { isFolder = true; return ''; });

  // -h → hide_in_views
  s = s.replace(/-h\b/gi, () => { hideInViews = true; return ''; });

  // -o → subtasks_in_order
  s = s.replace(/-o\b/gi, () => { subtasksInOrder = true; return ''; });

  // -c=<Color> or -c<Color> → colorHex
  s = s.replace(/-c(?:=)?([a-z]+)/gi, (_, name) => {
    const hex = parseColorName(name);
    if (hex) colorHex = hex;
    return '';
  });

  // s:<expr> start date
  s = s.replace(/\bs:(\S+(?:\s+\S+)?)/gi, (_, expr) => {
    startDate = parseDateExpr(expr);
    return '';
  });

  // d:<expr> due date
  s = s.replace(/\bd:(\S+(?:\s+\S+)?)/gi, (_, expr) => {
    dueDate = parseDateExpr(expr);
    return '';
  });

  // -start <expr> or -s <expr> → start date
  s = s.replace(/-start\s+(\S+(?:\s+\S+)?)/gi, (_, expr) => {
    startDate = parseDateExpr(expr);
    return '';
  });
  s = s.replace(/-s\s+(\S+(?:\s+\S+)?)/gi, (_, expr) => {
    startDate = parseDateExpr(expr);
    return '';
  });

  // -due <expr> or -d <expr> → due date
  s = s.replace(/-due\s+(\S+(?:\s+\S+)?)/gi, (_, expr) => {
    dueDate = parseDateExpr(expr);
    return '';
  });
  s = s.replace(/-d\s+(\S+(?:\s+\S+)?)/gi, (_, expr) => {
    dueDate = parseDateExpr(expr);
    return '';
  });

  // 3. remind / rmd <expr>
  s = s.replace(/\b(?:remind(?:\s+me)?|rmd)\s+(.+)/i, (_, expr) => {
    reminderAt = parseReminderExpr(expr.trim(), dueDate);
    return '';
  });

  // 4. NLP date from right side — only if no explicit s:/d: found
  if (!dueDate && !startDate) {
    const { text, date } = extractNlpDate(s);
    if (date) { dueDate = date; s = text; }
  }

  const caption = quotedCaption ?? s.replace(/\s+/g, ' ').trim();
  const result: ParsedTask = { caption: caption || raw.trim(), flagId, tagIds, starred, startDate, dueDate, reminderAt };
  if (importance !== undefined) result.importance = importance;
  if (urgency !== undefined) result.urgency = urgency;
  if (effort !== undefined) result.effort = effort;
  if (timeRequired !== undefined) result.timeRequired = timeRequired;
  if (timeRequiredMax !== undefined) result.timeRequiredMax = timeRequiredMax;
  if (leadTime !== undefined) result.leadTime = leadTime;
  if (colorHex !== undefined) result.colorHex = colorHex;
  if (hideInViews !== undefined) result.hideInViews = hideInViews;
  if (subtasksInOrder !== undefined) result.subtasksInOrder = subtasksInOrder;
  if (isProject !== undefined) result.isProject = isProject;
  if (isFolder !== undefined) result.isFolder = isFolder;
  return result;
}

// ── Date expression parser ────────────────────────────────────────────────────

export function parseDateExpr(expr: string): string | null {
  if (!expr) return null;
  expr = expr.trim().toLowerCase();

  const today = new Date();
  today.setHours(0, 0, 0, 0);

  // ISO: 2026-01-15
  if (/^\d{4}-\d{2}-\d{2}$/.test(expr)) {
    return expr;
  }

  // +Nd / +Nw
  const relShort = expr.match(/^\+(\d+)([dw])$/);
  if (relShort) {
    const n = parseInt(relShort[1]);
    const unit = relShort[2];
    const d = addDays(today, unit === 'w' ? n * 7 : n);
    return fmtDate(d);
  }

  // now → current datetime
  if (expr === 'now') {
    const n = new Date();
    return `${fmtDate(n)}T${String(n.getHours()).padStart(2,'0')}:${String(n.getMinutes()).padStart(2,'0')}`;
  }

  // Extract time suffix
  let time: string | null = null;
  let rest = expr;
  const timePat = /\b(?:at\s+)?(\d{1,2})(?::(\d{2}))?\s*(am|pm)$|(\d{2}):(\d{2})$/i;
  const timeMatch = rest.match(timePat);
  if (timeMatch) {
    rest = rest.slice(0, rest.length - timeMatch[0].length).trim();
    if (timeMatch[4]) {
      time = `${timeMatch[4]}:${timeMatch[5]}`;
    } else {
      let h = parseInt(timeMatch[1]);
      const m = timeMatch[2] ? parseInt(timeMatch[2]) : 0;
      const ap = timeMatch[3]?.toLowerCase();
      if (ap === 'pm' && h < 12) h += 12;
      if (ap === 'am' && h === 12) h = 0;
      time = `${String(h).padStart(2,'0')}:${String(m).padStart(2,'0')}`;
    }
  }

  let date: Date | null = null;

  if (rest === 'today') date = new Date(today);
  else if (rest === 'tomorrow') date = addDays(today, 1);
  else if (rest === 'yesterday') date = addDays(today, -1);
  else if (/^next\s+/.test(rest)) {
    const afterNext = rest.replace(/^next\s+/, '');
    // "next year"
    if (afterNext === 'year') {
      date = new Date(today.getFullYear() + 1, 0, 1);
    } else {
      const wd = parseWeekday(afterNext);
      if (wd !== null) date = nextWeekday(today, wd, true);
    }
  } else {
    const wd = parseWeekday(rest);
    if (wd !== null) date = nextWeekday(today, wd, false);
  }

  // today in 1h 25min — "today" anchor with hour/min offset
  if (!date && /^today\s+in\s+/i.test(rest)) {
    const offsetPart = rest.replace(/^today\s+in\s+/i, '');
    const mins = parseHourMinOffset(offsetPart);
    if (mins !== null) {
      const n = new Date();
      n.setSeconds(0, 0);
      n.setMinutes(n.getMinutes() + mins);
      return `${fmtDate(n)}T${String(n.getHours()).padStart(2,'0')}:${String(n.getMinutes()).padStart(2,'0')}`;
    }
  }

  // in N min / in N minutes → relative minutes from now
  if (!date) {
    const inMinMatch = rest.match(/^in\s+(\d+)\s*min(?:utes?)?$/i);
    if (inMinMatch) {
      const n = new Date();
      n.setSeconds(0, 0);
      n.setMinutes(n.getMinutes() + parseInt(inMinMatch[1]));
      return `${fmtDate(n)}T${String(n.getHours()).padStart(2,'0')}:${String(n.getMinutes()).padStart(2,'0')}`;
    }
  }

  // in N years / in Ny
  if (!date) {
    const inYearMatch = rest.match(/^in\s+(\d+)\s*y(?:ears?)?$/i);
    if (inYearMatch) {
      date = new Date(today);
      date.setFullYear(date.getFullYear() + parseInt(inYearMatch[1]));
    }
  }

  // in N days/weeks/months (simple single-unit)
  if (!date) {
    const inMatch = rest.match(/^in\s+(\d+)\s+(day|week|month)s?$/i);
    if (inMatch) {
      const n = parseInt(inMatch[1]);
      const unit = inMatch[2].toLowerCase();
      if (unit === 'day') date = addDays(today, n);
      else if (unit === 'week') date = addDays(today, n * 7);
      else if (unit === 'month') {
        date = new Date(today);
        date.setMonth(date.getMonth() + n);
      }
    }
  }

  // compound: in 2 months 1 week 4 days / in 1 m 2 w 1 d
  if (!date) {
    const compoundMatch = rest.match(/^in\s+(.+)$/i);
    if (compoundMatch) {
      const parts = compoundMatch[1];
      let months = 0, weeks = 0, days = 0;
      const mM = parts.match(/(\d+)\s*m(?:onths?)?(?:\s|$)/i);
      const mW = parts.match(/(\d+)\s*w(?:eeks?)?(?:\s|$)/i);
      const mD = parts.match(/(\d+)\s*d(?:ays?)?(?:\s|$)/i);
      if (mM) months = parseInt(mM[1]);
      if (mW) weeks = parseInt(mW[1]);
      if (mD) days = parseInt(mD[1]);
      if (months || weeks || days) {
        date = new Date(today);
        if (months) date.setMonth(date.getMonth() + months);
        if (weeks) date = addDays(date, weeks * 7);
        if (days) date = addDays(date, days);
      }
    }
  }

  // in N weeks <Weekday> — find weekday after N weeks
  if (!date) {
    const inWeeksWdMatch = rest.match(/^in\s+(\d+)\s+weeks?\s+(\w+)$/i);
    if (inWeeksWdMatch) {
      const n = parseInt(inWeeksWdMatch[1]);
      const wd = parseWeekday(inWeeksWdMatch[2]);
      if (wd !== null) {
        const anchor = addDays(today, n * 7);
        date = nextWeekday(anchor, wd, false);
      }
    }
  }

  // <Weekday> <HH:mm> — e.g. "Tue 11:20" (time already extracted above; rest is just weekday)
  if (!date) {
    const wdOnly = parseWeekday(rest);
    if (wdOnly !== null) date = nextWeekday(today, wdOnly, false);
  }

  // Month name + day (with optional ordinal suffix): jan 26, january 26, August 26th, Nov 26 08
  if (!date) {
    const monMatch = rest.match(/^(jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\s+(\d{1,2})(?:st|nd|rd|th)?(?:\s+(\d{2,4}))?$/i);
    if (monMatch) {
      const monthIdx = parseMonthName(monMatch[1]);
      const day = parseInt(monMatch[2]);
      let year = today.getFullYear();
      if (monMatch[3]) {
        const y = parseInt(monMatch[3]);
        year = y < 100 ? 2000 + y : y;
      }
      date = new Date(year, monthIdx, day);
      if (!monMatch[3] && date <= today) date.setFullYear(date.getFullYear() + 1);
    }
  }

  // Jan26 / Aug26 compact form (no space)
  if (!date) {
    const compactMon = rest.match(/^(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)(\d{1,2})(?:st|nd|rd|th)?$/i);
    if (compactMon) {
      const monthIdx = parseMonthName(compactMon[1]);
      const day = parseInt(compactMon[2]);
      date = new Date(today.getFullYear(), monthIdx, day);
      if (date <= today) date.setFullYear(date.getFullYear() + 1);
    }
  }

  // MDY: 3-26-2008 or 3/26/2008
  if (!date) {
    const mdyMatch = rest.match(/^(\d{1,2})[-\/](\d{1,2})[-\/](\d{2,4})$/);
    if (mdyMatch) {
      const a = parseInt(mdyMatch[1]);
      const b = parseInt(mdyMatch[2]);
      let y = parseInt(mdyMatch[3]);
      if (y < 100) y += 2000;
      // Heuristic: if first number > 12, it must be day (DMY); else treat as MDY
      if (a > 12) {
        // DMY
        date = new Date(y, b - 1, a);
      } else {
        // MDY
        date = new Date(y, a - 1, b);
      }
    }
  }

  // 3/15 format (no year)
  if (!date) {
    const slashMatch = rest.match(/^(\d{1,2})\/(\d{1,2})$/);
    if (slashMatch) {
      const month = parseInt(slashMatch[1]) - 1;
      const day = parseInt(slashMatch[2]);
      date = new Date(today.getFullYear(), month, day);
      if (date <= today) date.setFullYear(date.getFullYear() + 1);
    }
  }

  if (!date) return null;

  if (time) return `${fmtDate(date)}T${time}`;
  return fmtDate(date);
}

// ── Reminder expression parser ────────────────────────────────────────────────

function parseReminderExpr(expr: string, dueDate: string | null): string | null {
  // "me [date expr]" or "me at [date]" — absolute
  if (/^me\b/i.test(expr)) {
    const rest = expr.replace(/^me\s*(at\s*)?/i, '').trim();
    if (!rest) return dueDate ?? null;
    return parseDateExpr(rest);
  }

  // "at [time/date]" — absolute shorthand
  if (/^at\s+/i.test(expr)) {
    return parseDateExpr(expr.replace(/^at\s+/i, '').trim());
  }

  // "N min/hour/day before/in advance" — relative to due date
  // If no due date, calculate relative to now instead
  const offsetMatch = expr.match(/(\d+)\s*(min(?:ute)?s?|h(?:our)?s?|d(?:ay)?s?)\s*(?:before|in\s+advance)?/i);
  if (offsetMatch) {
    const n = parseInt(offsetMatch[1]);
    const unit = offsetMatch[2].toLowerCase();
    // Prefer due date as anchor; fall back to now
    const anchor = dueDate
      ? (parseDateToMs(dueDate + (dueDate.length === 10 ? 'T09:00' : '')) ?? Date.now())
      : Date.now();
    let ms = anchor;
    if (unit.startsWith('min')) ms -= n * 60000;
    else if (unit.startsWith('h')) ms -= n * 3600000;
    else if (unit.startsWith('d')) ms -= n * 86400000;
    const d = new Date(ms);
    return `${fmtDate(d)}T${String(d.getHours()).padStart(2,'0')}:${String(d.getMinutes()).padStart(2,'0')}`;
  }

  return parseDateExpr(expr);
}

// ── NLP date extraction (right-to-left, for bare dates at end of string) ─────

function extractNlpDate(s: string): { text: string, date: string | null } {
  // Try matching from the right side of the string
  const patterns = [
    // "in N weeks Weekday"
    /\bin\s+\d+\s+weeks?\s+\w+\b/i,
    // "in N days", "in N weeks", "in N months", "in N years"
    /\bin\s+\d+\s+(?:day|week|month|year)s?\b/i,
    // "in N min/minutes"
    /\bin\s+\d+\s*min(?:utes?)?\b/i,
    // compound: "in 2 months 1 week 4 days"
    /\bin\s+(?:\d+\s*(?:months?|m)\s*)?(?:\d+\s*(?:weeks?|w)\s*)?(?:\d+\s*(?:days?|d))?\b/i,
    // Weekday possibly with "next"
    /\b(?:next\s+)?(?:monday|tuesday|wednesday|thursday|friday|saturday|sunday)\b/i,
    // "tomorrow", "today"
    /\btomorrow\b|\btoday\b/i,
    // month day (with optional ordinal)
    /\b(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\s+\d{1,2}(?:st|nd|rd|th)?\b/i,
    // ISO date
    /\b\d{4}-\d{2}-\d{2}\b/,
    // +Nd
    /\+\d+[dw]\b/,
  ];

  for (const pat of patterns) {
    const match = s.match(pat);
    if (match) {
      const date = parseDateExpr(match[0]);
      if (date) {
        const text = (s.slice(0, match.index!) + s.slice(match.index! + match[0].length)).trim();
        return { text, date };
      }
    }
  }
  return { text: s, date: null };
}

// ── Time duration parser (for -t, -l switches) ────────────────────────────────

function parseTimeExpr(s: string): number {
  // Formats: 10 → 10 min, 2h15min → 135, 2h15m → 135, 1h → 60, 30m → 30
  let mins = 0;
  const hMatch = s.match(/(\d+)\s*h/i);
  const mMatch = s.match(/(\d+)\s*m(?:in)?(?!\w)/i);
  if (hMatch) mins += parseInt(hMatch[1]) * 60;
  if (mMatch) mins += parseInt(mMatch[1]);
  // pure number → minutes
  if (!hMatch && !mMatch) mins = parseInt(s) || 0;
  return mins;
}

// ── Hour+minute offset parser (for "today in 1h 25min") ───────────────────────

function parseHourMinOffset(s: string): number | null {
  let mins = 0;
  let found = false;
  const hMatch = s.match(/(\d+)\s*h(?:ours?)?/i);
  const mMatch = s.match(/(\d+)\s*min(?:utes?)?/i);
  if (hMatch) { mins += parseInt(hMatch[1]) * 60; found = true; }
  if (mMatch) { mins += parseInt(mMatch[1]); found = true; }
  return found ? mins : null;
}

// ── Color name → hex ──────────────────────────────────────────────────────────

function parseColorName(name: string): string | null {
  const colors: Record<string, string> = {
    red: '#FF3333', green: '#33CC33', blue: '#3366FF',
    yellow: '#FFFF33', orange: '#FF9933', purple: '#9933FF',
    pink: '#FF33AA', cyan: '#33CCFF', grey: '#999999', gray: '#999999',
  };
  return colors[name.toLowerCase()] ?? null;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function findFlag(name: string, flags: Flag[]): Flag | null {
  const lower = name.toLowerCase();
  return flags.find(f => f.name.toLowerCase().includes(lower)) ?? null;
}

function findOrNoteTag(name: string, tags: Tag[], tagIds: string[]): void {
  const lower = name.toLowerCase();
  const tag = tags.find(t => t.name.toLowerCase() === lower);
  if (tag && !tagIds.includes(tag.id)) tagIds.push(tag.id);
}

function parseWeekday(s: string): number | null {
  const days: Record<string, number> = {
    mon: 1, monday: 1, tue: 2, tuesday: 2, wed: 3, wednesday: 3,
    thu: 4, thursday: 4, fri: 5, friday: 5, sat: 6, saturday: 6, sun: 0, sunday: 0,
  };
  return days[s.toLowerCase()] ?? null;
}

function nextWeekday(from: Date, wd: number, mustBeNext: boolean): Date {
  const d = new Date(from);
  d.setDate(d.getDate() + (mustBeNext ? 1 : 0));
  while (d.getDay() !== wd) d.setDate(d.getDate() + 1);
  return d;
}

function parseMonthName(s: string): number {
  const months: Record<string, number> = {
    jan: 0, january: 0, feb: 1, february: 1, mar: 2, march: 2,
    apr: 3, april: 3, may: 4, jun: 5, june: 5, jul: 6, july: 6,
    aug: 7, august: 7, sep: 8, september: 8, oct: 9, october: 9,
    nov: 10, november: 10, dec: 11, december: 11,
  };
  return months[s.slice(0, 3).toLowerCase()] ?? 0;
}

function addDays(d: Date, n: number): Date {
  const r = new Date(d);
  r.setDate(r.getDate() + n);
  return r;
}

function fmtDate(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;
}

function parseDateToMs(s: string): number | null {
  const d = new Date(s);
  return isNaN(d.getTime()) ? null : d.getTime();
}

// ── Rapid Input line parser ───────────────────────────────────────────────────

export interface RapidLine {
  depth: number;
  raw: string;
  parsed: ParsedTask;
}

export function parseRapidInput(text: string, flags: Flag[], tags: Tag[], applyParsing: boolean): RapidLine[] {
  const lines = text.split('\n').filter(l => l.trim());
  if (!lines.length) return [];

  // Detect indent unit
  const indentUnit = detectIndentUnit(lines);

  return lines.map(line => {
    const stripped = line.replace(/\t/g, '    ');
    const indent = stripped.length - stripped.trimStart().length;
    const depth = indentUnit > 0 ? Math.floor(indent / indentUnit) : 0;
    const raw = stripped.trimStart();
    const parsed = applyParsing ? parseCaption(raw, flags, tags) : { caption: raw, flagId: null, tagIds: [], starred: false, startDate: null, dueDate: null, reminderAt: null } as ParsedTask;
    return { depth, raw, parsed };
  });
}

function detectIndentUnit(lines: string[]): number {
  const indents = lines
    .map(l => l.replace(/\t/g, '    '))
    .map(l => l.length - l.trimStart().length)
    .filter(n => n > 0);
  if (!indents.length) return 4;
  return Math.min(...indents);
}

// ── Date formatting for display ────────────────────────────────────────────────

export function formatDateDisplay(s: string | null): string {
  if (!s) return '';
  const d = new Date(s.includes('T') ? s : s + 'T00:00:00');
  if (isNaN(d.getTime())) return s;
  const now = new Date();
  const months = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
  let out = `${months[d.getMonth()]} ${d.getDate()}`;
  if (d.getFullYear() !== now.getFullYear()) out += ` ${d.getFullYear()}`;
  if (s.includes('T')) {
    out += ` ${String(d.getHours()).padStart(2,'0')}:${String(d.getMinutes()).padStart(2,'0')}`;
  }
  return out;
}

export function dateClass(s: string | null): string {
  if (!s) return '';
  const d = new Date(s.includes('T') ? s : s + 'T00:00:00');
  const diff = Math.floor((d.getTime() - Date.now()) / 86400000);
  if (diff < 0) return 'overdue';
  if (diff === 0) return 'today';
  return '';
}
