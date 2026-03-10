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

  // #tag or @tag
  s = s.replace(/[#@](\w+)/g, (_, name) => {
    const tag = findOrNoteTag(name, tags, tagIds);
    return '';
  });

  // * standalone star
  s = s.replace(/\b\*\b/g, () => { starred = true; return ''; });
  // -star or -*
  s = s.replace(/-star\b|-\*/gi, () => { starred = true; return ''; });

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

  // -s <expr> start; -d <expr> due
  s = s.replace(/-s\s+(\S+(?:\s+\S+)?)/gi, (_, expr) => {
    startDate = parseDateExpr(expr);
    return '';
  });
  s = s.replace(/-d\s+(\S+(?:\s+\S+)?)/gi, (_, expr) => {
    dueDate = parseDateExpr(expr);
    return '';
  });

  // 3. remind / rmd <expr>
  s = s.replace(/\b(?:remind|rmd)\s+(.+)/i, (_, expr) => {
    reminderAt = parseReminderExpr(expr.trim(), dueDate);
    return '';
  });

  // 4. NLP date from right side — only if no explicit s:/d: found
  if (!dueDate && !startDate) {
    const { text, date } = extractNlpDate(s);
    if (date) { dueDate = date; s = text; }
  }

  const caption = quotedCaption ?? s.replace(/\s+/g, ' ').trim();
  return { caption: caption || raw.trim(), flagId, tagIds, starred, startDate, dueDate, reminderAt };
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
    const day = rest.replace(/^next\s+/, '');
    const wd = parseWeekday(day);
    if (wd !== null) date = nextWeekday(today, wd, true);
  } else {
    const wd = parseWeekday(rest);
    if (wd !== null) date = nextWeekday(today, wd, false);
  }

  // in N days/weeks/months
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

  // Month name + day: jan 26, january 26
  if (!date) {
    const monMatch = rest.match(/^(jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\s+(\d{1,2})$/i);
    if (monMatch) {
      const monthIdx = parseMonthName(monMatch[1]);
      const day = parseInt(monMatch[2]);
      date = new Date(today.getFullYear(), monthIdx, day);
      if (date <= today) date.setFullYear(date.getFullYear() + 1);
    }
  }

  // 3/15 format
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
  // "me [date expr]" — absolute
  if (expr.startsWith('me')) {
    const rest = expr.slice(2).trim();
    if (!rest) return dueDate ?? null;
    return parseDateExpr(rest);
  }

  // "N min/hour before/in advance"
  const offsetMatch = expr.match(/(\d+)\s*(min(?:ute)?s?|h(?:our)?s?|d(?:ay)?s?)\s*(?:before|in\s+advance)?/i);
  if (offsetMatch && dueDate) {
    const n = parseInt(offsetMatch[1]);
    const unit = offsetMatch[2].toLowerCase();
    const due = parseDateToMs(dueDate);
    if (due) {
      let ms = due;
      if (unit.startsWith('min')) ms -= n * 60000;
      else if (unit.startsWith('h')) ms -= n * 3600000;
      else if (unit.startsWith('d')) ms -= n * 86400000;
      const d = new Date(ms);
      return `${fmtDate(d)}T${String(d.getHours()).padStart(2,'0')}:${String(d.getMinutes()).padStart(2,'0')}`;
    }
  }

  return parseDateExpr(expr);
}

// ── NLP date extraction (right-to-left, for bare dates at end of string) ─────

function extractNlpDate(s: string): { text: string, date: string | null } {
  // Try matching from the right side of the string
  const patterns = [
    // "in N days", "in N weeks", etc.
    /\bin\s+\d+\s+(?:day|week|month)s?\b/i,
    // Weekday possibly with "next"
    /\b(?:next\s+)?(?:monday|tuesday|wednesday|thursday|friday|saturday|sunday)\b/i,
    // "tomorrow", "today"
    /\btomorrow\b|\btoday\b/i,
    // month day
    /\b(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\s+\d{1,2}\b/i,
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
    const parsed = applyParsing ? parseCaption(raw, flags, tags) : { caption: raw, flagId: null, tagIds: [], starred: false, startDate: null, dueDate: null, reminderAt: null };
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
