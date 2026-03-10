import type { Flag, Tag } from './types';

// ── Parsed result from a single line ──────────────────────────────────────────
export interface ParsedTask {
  caption: string;
  flagId?: string;
  tagIds: string[];
  startDate?: string;
  dueDate?: string;
  reminderAt?: string;
  starred: boolean;
}

// ── Node in the hierarchy tree ────────────────────────────────────────────────
export interface ParsedNode {
  parsed: ParsedTask;
  raw: string;         // original trimmed line, for display when parsing is off
  children: ParsedNode[];
}

// ── Date parser ───────────────────────────────────────────────────────────────
const WEEKDAYS = ['sun','mon','tue','wed','thu','fri','sat'];
const WEEKDAYS_FULL = ['sunday','monday','tuesday','wednesday','thursday','friday','saturday'];

function fmt(d: Date): string {
  return d.toISOString().slice(0, 10);
}

export function parseDate(token: string): string | undefined {
  const t = token.toLowerCase().trim();
  const today = new Date();
  today.setHours(0, 0, 0, 0);

  // Named shortcuts
  if (t === 'today' || t === 'tod') return fmt(today);
  if (t === 'tomorrow' || t === 'tom') {
    const d = new Date(today); d.setDate(d.getDate() + 1); return fmt(d);
  }
  if (t === 'nextweek' || t === 'nw' || t === 'next-week') {
    const d = new Date(today);
    const daysUntilMon = (8 - d.getDay()) % 7 || 7;
    d.setDate(d.getDate() + daysUntilMon);
    return fmt(d);
  }

  // Weekday names: next occurrence
  const wdShort = WEEKDAYS.indexOf(t.slice(0, 3));
  const wdFull  = WEEKDAYS_FULL.indexOf(t);
  const wd = wdShort !== -1 ? wdShort : wdFull !== -1 ? wdFull : -1;
  if (wd !== -1) {
    const d = new Date(today);
    const diff = (wd - d.getDay() + 7) % 7 || 7;
    d.setDate(d.getDate() + diff);
    return fmt(d);
  }

  // Relative: +Nd / +Nw / +Nm
  const rel = t.match(/^\+(\d+)([dwm])$/);
  if (rel) {
    const n = parseInt(rel[1]);
    const d = new Date(today);
    if (rel[2] === 'd') d.setDate(d.getDate() + n);
    else if (rel[2] === 'w') d.setDate(d.getDate() + n * 7);
    else if (rel[2] === 'm') d.setMonth(d.getMonth() + n);
    return fmt(d);
  }

  // ISO: YYYY-MM-DD
  if (/^\d{4}-\d{2}-\d{2}$/.test(t)) return t;

  // DD/MM/YY or DD/MM/YYYY or DD-MM-YYYY
  const dmy = t.match(/^(\d{1,2})[\/\-](\d{1,2})[\/\-](\d{2,4})$/);
  if (dmy) {
    const day   = dmy[1].padStart(2, '0');
    const month = dmy[2].padStart(2, '0');
    const year  = dmy[3].length === 2 ? '20' + dmy[3] : dmy[3];
    const iso = `${year}-${month}-${day}`;
    if (!isNaN(new Date(iso).getTime())) return iso;
  }

  return undefined;
}

// ── Line parser ───────────────────────────────────────────────────────────────
const DATE_PREFIXES = /^(s|start|d|due|r|rem|reminder):(.+)$/i;

export function parseTaskLine(raw: string, flags: Flag[], tags: Tag[]): ParsedTask {
  const words = raw.trim().split(/\s+/);
  const captionWords: string[] = [];
  let flagId: string | undefined;
  const tagIds: string[] = [];
  let startDate: string | undefined;
  let dueDate:   string | undefined;
  let reminderAt: string | undefined;
  let starred = false;

  for (const word of words) {
    if (word === '*') {
      starred = true;
      continue;
    }

    if (word.startsWith('!') && word.length > 1) {
      const name = word.slice(1).toLowerCase();
      const flag = flags.find(f => f.name.toLowerCase().startsWith(name));
      if (flag) { flagId = flag.id; continue; }
    }

    if (word.startsWith('#') && word.length > 1) {
      const name = word.slice(1).toLowerCase();
      const tag = tags.find(t => t.name.toLowerCase() === name);
      if (tag) { tagIds.push(tag.id); continue; }
    }

    const dm = word.match(DATE_PREFIXES);
    if (dm) {
      const parsed = parseDate(dm[2]);
      if (parsed !== undefined) {
        const prefix = dm[1].toLowerCase();
        if (prefix === 's' || prefix === 'start')                    startDate  = parsed;
        else if (prefix === 'd' || prefix === 'due')                 dueDate    = parsed;
        else if (prefix === 'r' || prefix === 'rem' || prefix === 'reminder') reminderAt = parsed;
        continue;
      }
    }

    captionWords.push(word);
  }

  return {
    caption: captionWords.join(' '),
    flagId,
    tagIds,
    startDate,
    dueDate,
    reminderAt,
    starred,
  };
}

// ── Indent counter ────────────────────────────────────────────────────────────
function lineIndent(line: string): number {
  let count = 0;
  for (const ch of line) {
    if (ch === '\t') count += 4;
    else if (ch === ' ') count++;
    else break;
  }
  return count;
}

// ── Rapid input parser ────────────────────────────────────────────────────────
export function parseRapidInput(
  text: string,
  flags: Flag[],
  tags: Tag[],
  applyParsing: boolean
): ParsedNode[] {
  const rawLines = text.split('\n');
  const lines = rawLines
    .map(l => ({ raw: l.trimEnd(), indent: lineIndent(l), trimmed: l.trim() }))
    .filter(l => l.trimmed.length > 0);

  if (!lines.length) return [];

  // Normalise indent values to levels 0,1,2,...
  const uniqueIndents = [...new Set(lines.map(l => l.indent))].sort((a, b) => a - b);
  const toLevel = new Map(uniqueIndents.map((v, i) => [v, i]));

  const roots: ParsedNode[] = [];
  const stack: Array<{ node: ParsedNode; level: number }> = [];

  for (const { trimmed, indent } of lines) {
    const level = toLevel.get(indent) ?? 0;
    const parsed = applyParsing
      ? parseTaskLine(trimmed, flags, tags)
      : { caption: trimmed, tagIds: [], starred: false };
    const node: ParsedNode = { parsed, raw: trimmed, children: [] };

    while (stack.length > 0 && stack[stack.length - 1].level >= level) stack.pop();

    if (stack.length === 0) roots.push(node);
    else stack[stack.length - 1].node.children.push(node);

    stack.push({ node, level });
  }

  return roots;
}

// ── Flatten for display ───────────────────────────────────────────────────────
export function flattenNodes(
  nodes: ParsedNode[],
  depth = 0
): Array<{ node: ParsedNode; depth: number }> {
  const result: Array<{ node: ParsedNode; depth: number }> = [];
  for (const node of nodes) {
    result.push({ node, depth });
    if (node.children.length > 0) result.push(...flattenNodes(node.children, depth + 1));
  }
  return result;
}
