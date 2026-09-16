// Horae display formats — single source for hours, dates and money.
// hours(150)          → "2:30"       H:MM, no leading zero on hours, always two-digit minutes
// hoursLong(5047)     → "1:24:07"    running timer only
// clock(d)            → "09:12"      24h, two digits, no seconds
// date(d)             → "12 Jul 2026"
// dateShort(d)        → "12 Jul"     when the year is implied by context (same-week tables)
// dateTime(d)         → "12 Jul, 18:20"
// money(7200, 'EUR')  → "€7,200.00"  symbol always present; ISO code only when mixing currencies
const MON = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
const SYM = { EUR: '€', USD: '$', GBP: '£', CHF: 'CHF ' };
const p2 = (n) => String(n).padStart(2, '0');
export const hours = (min) => `${Math.floor(min / 60)}:${p2(Math.round(min % 60))}`;
export const hoursLong = (sec) => `${Math.floor(sec / 3600)}:${p2(Math.floor(sec / 60) % 60)}:${p2(sec % 60)}`;
export const clock = (d) => `${p2(d.getHours())}:${p2(d.getMinutes())}`;
export const date = (d) => `${p2(d.getDate())} ${MON[d.getMonth()]} ${d.getFullYear()}`;
export const dateShort = (d) => `${p2(d.getDate())} ${MON[d.getMonth()]}`;
export const dateTime = (d) => `${dateShort(d)}, ${clock(d)}`;
export const range = (a, b) => a.getMonth() === b.getMonth()
  ? `${p2(a.getDate())} – ${p2(b.getDate())} ${MON[b.getMonth()]} ${b.getFullYear()}`
  : `${dateShort(a)} – ${dateShort(b)} ${b.getFullYear()}`;
export const money = (n, cur = 'EUR', showCode = false) =>
  `${SYM[cur] ?? ''}${n.toLocaleString('en-GB', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}${showCode ? ' ' + cur : ''}`;
