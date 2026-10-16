# Payroll: US gross-to-net (federal + WA + TX) as an optional crate

## Context

The goal is a fully compliant US payroll for Horae: approved Horae timesheets become pay runs. Each pay run does federal gross-to-net (FIT per Pub 15-T 2026, FICA, Additional Medicare, FUTA) plus the Washington and Texas state obligations. It produces pay stubs, a NACHA direct-deposit file, and the filing data the operator submits (941/Schedule B, 940/Schedule A, W-2 copies, EFW2, the WA ESD/PFML/L&I and TX TWC quarterly reports, deposit due dates, and new-hire reports). Nothing is transmitted to agencies.

The payroll engine lives in a **separate pure crate** (`crates/payroll`, package `horae-payroll`). It has a serde-in/serde-out API and no I/O, so it can later be wrapped as an Extism WASM plugin. The app includes it only through a Cargo feature `payroll`. Storage, UI, downloads, and the approval-lock integration live in the host app behind that feature.

A new NixOS VM test, `nix/checks/e2e-payroll.nix`, proves the integration end to end:
1. Real timesheet submit and approve.
2. A pay run with hand-computed expected figures.
3. Finalize.
4. The NACHA file, 941 and EFW2 output.
5. The approval lock.

## Compliance requirements this plan implements (research)

| Requirement | Source | Implemented in |
|---|---|---|
| FIT: percentage method, Worksheet 1A, 2026 annual tables (STANDARD + Step-2 checkbox), $8,600/$12,900 line 1g, $4,300 per pre-2020 allowance, Table 3 periods (weekly 52, biweekly 26); unsubmitted W-4 → Single, no adjustments | IRS Pub 15-T (2026) https://www.irs.gov/publications/p15t, Pub 15 | Step 2 `federal::fit` |
| SS 6.2%/6.2%, wage base $184,500; Medicare 1.45%/1.45%; Additional Medicare 0.9% on employee wages > $200,000 YTD, no employer share | Pub 15 (2026) https://www.irs.gov/publications/p15 | Step 2 `federal::fica` |
| FUTA 6.0% − 5.4% credit = 0.6% on first $7,000; Section 125 exempt, 401(k) deferrals not exempt; Schedule A for multi-state employers | Pub 15, Form 940 instructions | Steps 2–3 |
| Qualified overtime compensation (the FLSA "half" premium) reported on 2026 W-2 box 12 code TT | Pub 15 (P.L. 119-21), 2026 W-2/W-3 instructions https://www.irs.gov/pub/irs-pdf/iw2w3.pdf | Steps 2–3 |
| FLSA: 1.5× regular rate above 40h per fixed workweek, no averaging across weeks; records: name, SSN, address, birth date if <19, workweek start, hours per day/week, pay basis, regular rate, straight-time and OT earnings, additions/deductions, total wages, pay date and period; retain payroll 3 years, time records 2 years | DOL Fact Sheets #21/#23, 29 CFR 516 | Steps 2, 6, 7 |
| Federal min wage $7.25; EAP exempt salary ≥ $684/week | FLSA | Step 2 validation |
| WA min wage 2026 $17.13; WA EAP exempt salary ≥ $1,541.70/week (2026) | L&I | Step 2 validation |
| WA PFML 2026: 1.13% on wages up to SS cap $184,500; employee share = wages × 0.0113 × 0.7143 (rounded to cents); employer = total − employee; employer share waived for <50 employees | ESD Employer Toolkit v25.1 (June 2026) | Step 2 `state::wa` |
| WA Cares 0.58%, no cap, employee-paid, per-employee exemption | same | Step 2 `state::wa` |
| WA UI: employer rate per ESD notice, taxable base 2026 $78,200; quarterly report per employee: name, SSN, gross wages, hours (rounded **up**), SOC code or job title | ESD UI Tax Handbook (Apr 2026) | Steps 2–3 |
| WA L&I workers' comp: hourly rates per risk class, employee portion withheld per hour worked, quarterly hours by class | L&I rate notice F225-004-000 | Steps 2–3 |
| WA paid sick leave: ≥1h per 40h worked for non-exempt employees, 40h carryover cap, employee notice of accrual/use/balance | RCW 49.46.210 | Steps 2, 7, 10 |
| WA itemized pay statement: hours/days worked, rate(s), gross, all deductions, pay period (m/d/y), pay date | WAC 296-126-040 | Step 10 stub |
| TX UI: employer rate per TWC notice, base $9,000; quarterly C-3 wage list; itemized earnings statement | TWC, Tex. Lab. Code 61.017 (statute text unverified — stub content is the superset below) | Steps 2, 3, 10 |
| 941 quarterly (line 1 uses the pay period containing Mar/Jun/Sep/Dec 12), monthly vs semiweekly deposit schedule, $100k next-day rule, legal-holiday-adjusted due dates | Pub 15 §11–12 | Step 3 |
| W-2 e-filing (EFW2 records RA/RE/RW/RT/RF, 512 chars) and employee copies | SSA Pub 42-007 EFW2 TY2026 https://www.ssa.gov/employer/efw/26efw2.pdf | Steps 3, 10 |
| New-hire reporting within 20 days (WA, TX) | ESD handbook | Step 10 |
| Employment tax records kept ≥4 years | IRS / ESD | Step 6 (immutable finalized rows, `ON DELETE RESTRICT`) |

## Approach

The steps are ordered so the tree builds and the existing suites pass after each one. Steps 2–3 (the engine) are independent of steps 4–8 (the host) apart from the crate dependency, so they can proceed in parallel once step 1 lands.

### 0. Check the plan into the repo (do this first, on its own)

- Create the directory `crates/payroll/`. Copy this plan file verbatim to `crates/payroll/PLAN.md`; that is the payroll crate's home and the plan's durable location.
- Do **not** add `crates/payroll` to the workspace `members` yet. A member without a `Cargo.toml` breaks every cargo command; step 1 adds both together.
- Commit only that file on the current branch: `git add crates/payroll/PLAN.md && git commit -m "Add payroll implementation plan"`. If the branch has an upstream (`git rev-parse --abbrev-ref @{u}` succeeds), push with `git push`. If it has none, stop after the commit and tell the user the commit exists only locally, because pushing needs a remote/branch choice.
- Later steps keep `crates/payroll/PLAN.md` as-is; it is not part of the Cargo package's build.

### 1. Workspace, feature and build wiring

- Root `Cargo.toml`: add `"crates/payroll"` to `members`.
- Create `crates/payroll/Cargo.toml`: `name = "horae-payroll"`, edition 2024, license MIT. Deps: `serde` (derive), `chrono` (`default-features = false`, features `["serde"]`), `uuid` (`features = ["serde"]`), `thiserror = "2"`. Dev-dep: `serde_json`.
  - **No** dependency on `horae-core`, sqlx, axum, dioxus, or anything that does I/O, time, or randomness. All dates come in as inputs. This is the WASM-plugin boundary; state it in the `lib.rs` doc comment.
- `crates/horae/Cargo.toml`:
  - Add `horae-payroll = { path = "../payroll", optional = true }`.
  - Add the feature `payroll = ["server", "dep:horae-payroll"]`.
- `nix/package.nix`: change `dxBuild` to `(cd crates/horae && dx build --release --locked --features payroll)`. The shipped package includes payroll; deployments activate it at runtime via `HORAE_PAYROLL_ENC_KEY` (step 5). In `checkArtifacts`, use `--features server,payroll` for both commands.
- `nix/checks/clippy.nix`: add these commands and keep the existing `--features server` line, which proves the feature-off build:
  - `cargo clippy -p horae-payroll --all-targets -- -D warnings -W clippy::perf`
  - `cargo clippy -p horae --features server,payroll --all-targets -- -D warnings -W clippy::perf`
  - `cargo check -p horae-payroll --target wasm32-unknown-unknown` (plugin-compatibility proof; the fenix toolchain already ships that target)
- `nix/checks/tests.nix`:
  - Add `cargo nextest run -p horae-payroll --test-threads "$NIX_BUILD_CORES"` next to the core line.
  - Change the DB line to `cargo test -p horae --features server,payroll -- ...`.
- `nix/checks/sqlx-prepare.nix` and the AGENTS.md "sqlx query cache" command: use `--features server,payroll`. Without it, prepare drops the payroll queries from `.sqlx/`.
- `process-compose.yaml` `app`: use `dx serve --interactive false --port 8080 --features payroll`. Add `- "HORAE_PAYROLL_ENC_KEY=00000000000000000000000000000000000000000000000000000000000000aa"` to `environment` (dev-only key, beside `DEV_LOGIN=1`).
- Contingency: if `dx build --features payroll` does not reach the server build in dx 0.7, use `dx build --release --locked @server --features payroll`. The client never needs the feature (step 9 keeps every client-visible item feature-independent).

### 2. Engine: gross-to-net (`crates/payroll/src/`)

Use the module layout `foo.rs` + `foo/`, never `mod.rs`. All money is `i64` cents (USD only, so no currency field), all durations are `i32` minutes, and all rates are integers.

- `money.rs`: `pub fn mul_div(cents: i64, num: i64, den: i64) -> i64`. Compute in i128 and round half away from zero to the nearest cent. Every percentage below goes through it. A `bp` rate is in hundredths of a percent (1.13% = 113). A `tt` rate is in ten-thousandths of a dollar per hour (L&I $0.2000/h = 2000).
- `year.rs`: `pub struct TaxYear(i32)` and `pub fn tables(year: i32) -> Result<&'static YearTables, PayrollError>`. Only 2026 exists; any other year returns `PayrollError::UnsupportedTaxYear(year)`, with the message `"No payroll tax tables for {year}"`. `YearTables` holds the following constants, all in `year/y2026.rs`:
  - FIT schedules: `[Bracket { at_least_cents, base_cents, rate_bp }]` for STANDARD and CHECKBOX × {Single, MarriedJointly, HeadOfHousehold}, transcribed **exactly** from the 2026 Pub 15-T annual table:
    - STANDARD Single: 0/0/0, 7,500/0/10%, 19,900/1,240/12%, 57,900/5,800/22%, 113,200/17,966/24%, 209,275/41,024/32%, 263,725/58,448/35%, 648,100/192,979.25/37%
    - STANDARD MFJ: 0/0/0, 19,300/0/10%, 44,100/2,480/12%, 120,100/11,600/22%, 230,700/35,932/24%, 422,850/82,048/32%, 531,750/116,896/35%, 788,000/206,583.50/37%
    - STANDARD HoH: 0/0/0, 15,550/0/10%, 33,250/1,770/12%, 83,000/7,740/22%, 121,250/16,155/24%, 217,300/39,207/32%, 271,750/56,631/35%, 656,150/191,171/37%
    - CHECKBOX Single: 0/0/0, 8,050/0/10%, 14,250/620/12%, 33,250/2,900/22%, 60,900/8,983/24%, 108,938/20,512/32%, 136,163/29,224/35%, 328,350/96,489.63/37%
    - CHECKBOX MFJ: 0/0/0, 16,100/0/10%, 28,500/1,240/12%, 66,500/5,800/22%, 121,800/17,966/24%, 217,875/41,024/32%, 272,325/58,448/35%, 400,450/103,291.75/37%
    - CHECKBOX HoH: 0/0/0, 12,075/0/10%, 20,925/885/12%, 45,800/3,870/22%, 64,925/8,077.50/24%, 112,950/19,603.50/32%, 140,175/28,315.50/35%, 332,375/95,585.50/37%
  - `line_1g_cents`: MFJ 1,290,000, otherwise 860,000. `pre2020_allowance_cents` 430,000.
  - `ss_rate_bp` 620, `ss_wage_base_cents` 18,450,000, `medicare_rate_bp` 145, `addl_medicare_rate_bp` 90, `addl_medicare_threshold_cents` 20,000,000.
  - `futa_net_rate_bp` 60, `futa_wage_base_cents` 700,000, `futa_credit_reduction_bp(state)` = 0 for WA and TX. Comment: "DOL announces credit-reduction states each November; update before filing 940".
  - `federal_min_wage_cents` 725, `federal_exempt_weekly_salary_cents` 68,400.
  - `legal_holidays`: 2026-01-01, 01-19, 02-16, 04-16 (DC Emancipation Day), 05-25, 06-19, 07-03, 09-07, 10-12, 11-11, 11-26, 12-25.
  - Per-state constants live in the state modules below.
- `types.rs` (all `Serialize + Deserialize`, `#[serde(rename_all = "snake_case")]`):
  - `WorkState { Wa, Tx }` (serde names `"WA"`/`"TX"`)
  - `PayFrequency { Weekly, Biweekly }` with `periods_per_year()` returning 52/26
  - `PayBasis { Hourly, Salary }`
  - `FilingStatus { Single, MarriedJointly, HeadOfHousehold }`
  - `W4 { form: W4Form { Y2020OrLater, Pre2020 }, filing_status, multiple_jobs: bool, dependents_cents, other_income_cents, deductions_cents, extra_withholding_cents, allowances: i32, exempt: bool }`
  - `DeductionKind { Retirement401k, Section125, PostTax }` and `Deduction { kind, description: String, amount_cents }`
  - `LineCode` — the closed set of line codes. Serde names are load-bearing because they are stored in the DB:
    - earnings: `regular`, `overtime`, `salary`, `sick_leave`
    - pre-tax: `ded_401k`, `ded_s125`
    - employee taxes: `fit`, `ss_ee`, `medicare_ee`, `addl_medicare`, `wa_pfml_ee`, `wa_cares`, `wa_li_ee`
    - post-tax: `ded_post`
    - employer: `ss_er`, `medicare_er`, `futa`, `suta`, `wa_pfml_er`, `wa_li_er`
  - `Line { code: LineCode, description: String, minutes: Option<i32>, rate_cents: Option<i64>, amount_cents: i64 }`. `LineCode::is_employer()` decides employer-paid.
  - `Issue { message: String }`
- `paycheck.rs`, the single entry point: `pub fn compute_paycheck(input: &PaycheckInput) -> Result<PaycheckResult, Vec<Issue>>`.
  - `PaycheckInput { pay_date, period_start, period_end, frequency, overtime_threshold_minutes: i32, employee: EmployeeProfile, weeks: Vec<WeekMinutes { week_start: NaiveDate, worked_minutes: i32 }>, sick_minutes: i32, employed_days: i32, ytd: Ytd, employer: EmployerRates }`
  - `EmployeeProfile { work_state, flsa_exempt, basis, rate_cents, w4: Option<W4>, deductions: Vec<Deduction>, wa_cares_exempt, li_rate: Option<LiRate { employer_tt: i64, employee_tt: i64 }> }`
  - `Ytd { gross_cents, ss_wages_cents, medicare_wages_cents, futa_wages_cents, suta_wages_cents, pfml_wages_cents, worked_minutes, sick_accrued_minutes, sick_used_minutes, sick_carryover_minutes }` — calendar year of `pay_date`; `suta_wages_cents` covers the same state only.
  - `EmployerRates { ui_rate_bp: i32, pfml_employer_exempt: bool }`
  - `PaycheckResult { lines: Vec<Line>, gross_cents, net_cents, regular_minutes, overtime_minutes, worked_minutes, qualified_overtime_cents, sick_accrued_minutes, wages: WageBases { fit, ss, medicare, futa, suta, pfml } }`

  Algorithm, in this order:
  1. **Validation.** Collect every failure before returning `Err`:
     - year tables exist
     - `pay_date >= period_end`
     - `1 <= overtime_threshold_minutes <= 2400`
     - `basis == Salary` ⇒ `flsa_exempt`, else "Salaried employees must be overtime-exempt; pay non-exempt staff hourly"
     - hourly `rate_cents >= max(federal, state)` minimum wage
     - salaried exempt `rate_cents / 52 >= max(federal, state)` exempt weekly salary (compare `rate_cents >= threshold × 52`)
     - `w4.form == Pre2020` ⇒ `filing_status != HeadOfHousehold`
     - WA ⇒ `li_rate.is_some()`
     - `sick_minutes` ≤ the available balance (WA non-exempt) or `0` otherwise
     - net ≥ 0, checked after computation
  2. **Hours and earnings** (hourly):
     - for each week: `ot = max(0, worked − threshold)`, `reg = worked − ot`
     - `regular = mul_div(Σreg, rate, 60)`, `overtime = mul_div(Σot, rate × 3, 120)`, `qualified_overtime = mul_div(Σot, rate, 120)`
     - `sick_leave = mul_div(sick_minutes, rate, 60)` (WA non-exempt only)

     Salaried: `salary = mul_div(rate_cents, employed_days, periods_per_year × days_in_period)`. This is a full period when `employed_days` = days in period (7 or 14). No overtime and no sick pay line; worked minutes are still summed.
  3. **Pre-tax deductions** from `employee.deductions` → `ded_401k` / `ded_s125` lines.
  4. **Wage bases.** All are capped at their YTD base remainder where a base exists.
     - `fit = gross − 401k − s125`; `ss = medicare = futa = gross − s125`
     - `pfml = gross` (WA)
     - `suta = gross` (WA) or `gross − s125` (TX; unverified — see Assumptions)
  5. **FIT** (`federal/fit.rs`), Worksheet 1A literally:
     - `exempt` ⇒ 0; missing W-4 ⇒ `Y2020OrLater Single` with zeros
     - `1c = fit × periods`
     - Y2020: `1i = max(0, 1c + other_income − (deductions + (multiple_jobs ? 0 : line_1g)))`. Pre2020: `1l = max(0, 1c − allowances × 4,300)` and use STANDARD.
     - table = `multiple_jobs ? CHECKBOX : STANDARD`
     - `2g = base + mul_div(2a − at_least, rate_bp, 10000)`
     - `2h = mul_div(2g, 1, periods)`, `3b = mul_div(dependents, 1, periods)`, `3c = max(0, 2h − 3b)`, `4b = 3c + extra`
  6. **FICA** (`federal/fica.rs`):
     - `ss_ee = ss_er = mul_div(min(ss, base − ytd_ss), 620, 10000)`
     - `medicare_ee = medicare_er = mul_div(medicare, 145, 10000)`
     - `addl_medicare = mul_div(max(0, ytd_med + medicare − max(ytd_med, 200,000.00)), 90, 10000)`
  7. **FUTA**: `mul_div(min(futa, 7,000 − ytd_futa), 60 + credit_reduction_bp, 10000)`.
  8. **State** (`state.rs`):
     - Trait `pub trait StateRules: Sync { fn min_wage_cents(&self, year: i32) -> i64; fn exempt_weekly_salary_cents(&self, year: i32) -> i64; fn suta_wage_base_cents(&self, year: i32) -> i64; fn sick_leave_accrues(&self, flsa_exempt: bool) -> bool; fn employee_and_employer_lines(&self, ctx: &StateCtx) -> Vec<Line>; }`
     - `pub fn rules(state: WorkState) -> &'static dyn StateRules`. Adding a state is a new module plus a `WorkState` variant plus the DB CHECK in step 6.
     - `state/wa.rs`:
       - min $17.13 (1713); exempt 154,170
       - SUTA base 7,820,000
       - `wa_pfml_ee = mul_div(pfml, 113 × 7143, 100_000_000)`; `wa_pfml_er = mul_div(pfml, 113, 10000) − ee` (0 if `pfml_employer_exempt`)
       - `wa_cares = wa_cares_exempt ? 0 : mul_div(gross, 58, 10000)`
       - `wa_li_ee = mul_div(worked + sick_minutes, employee_tt, 6000)`; `wa_li_er = mul_div(worked + sick_minutes, employer_tt, 6000)`
       - `suta = mul_div(suta_base, ui_rate_bp, 10000)`
       - sick leave accrues iff not exempt
     - `state/tx.rs`: min 725, exempt = federal 68,400, SUTA base 900,000, `suta` line only, no sick leave.
  9. **Post-tax** `ded_post` lines. `net = gross − pre-tax − employee taxes − post-tax`.
  10. **Sick accrual** (WA non-exempt): `floor((ytd.worked + worked)/40) − floor(ytd.worked/40)` minutes. Available balance = `carryover + ytd_accrued − ytd_used`. Carryover is computed by the host (step 7).
- `overtime.rs` is only the per-week split helper used in item 2. No averaging across weeks.
- Unit tests (`crates/payroll/src/paycheck/tests.rs`) pin these exact figures (they are reused by the NixOS test):
  - **WA hourly**: $30/h, 2,700 min in week 2026-01-05, threshold 2,400, W-4 2020 Single, no adjustments, 401(k) $50.00, L&I ee 2000tt / er 10000tt, UI 100bp, pay 2026-01-16.
    - regular 120000, overtime 22500, gross 142500, TT 7500
    - FIT wages 137500 → fit 13269
    - ss 8835 ×2, medicare 2066 ×2
    - wa_pfml_ee 1150, wa_pfml_er 460, wa_cares 827
    - wa_li_ee 900, wa_li_er 4500
    - futa 855, suta 1425
    - net **110453**, sick accrued 67
  - **TX salaried exempt**: $104,000/yr, weekly, W-4 2020 MFJ, dependents $2,000, UI 270bp.
    - salary 200000, fit 11769, ss 12400, medicare 2900, futa 1200, suta 5400
    - net **172931**
  - Issues: salaried non-exempt rejected; $15/h WA rejected (below 1713); year 2027 rejected with "No payroll tax tables for 2027".
  - Additional Medicare: YTD medicare 19,950,000 + 100,000 → addl 450.
  - SS cap: YTD ss 18,400,000 + 100,000 → ss on 50,000 only = 3100.

### 3. Engine: payment and filing outputs

All of these are pure functions in `crates/payroll/src/`, taking aggregated inputs and returning structs or strings.

- `nacha.rs`: `pub fn build(input: &NachaInput) -> Result<String, Vec<Issue>>`.
  - `NachaInput { odfi_routing: String /*9 digits*/, destination_name, company_name /*≤16*/, company_identification /*10*/, origin_name, effective_date: NaiveDate, created_at: NaiveDateTime, entries: Vec<NachaEntry { routing, account, account_type: AccountType { Checking, Savings }, amount_cents, individual_id /*≤15*/, name /*≤22*/ }> }`.
  - Validate routing checksums (weights 3,7,1; sum % 10 == 0), account ≤ 17 chars, amount > 0.
  - Emit 94-char records joined with `\n`:
    - File header `1`: priority `01`, ` `+ODFI routing, origin `1`+EIN-like `company_identification` (10), YYMMDD, HHMM, modifier `A`, `094`, `10`, `1`, names padded to 23, reference blank.
    - One batch `5`: service class `220`, SEC `PPD`, entry description `PAYROLL`, effective date, originator status `1`, ODFI first 8 digits, batch `0000001`.
    - Entries `6`: transaction code 22 (checking) / 32 (savings); trace = ODFI8 + 7-digit sequence.
    - Batch control `8`: entry hash = Σ first-8 routing digits mod 10^10; total debit 0; total credit.
    - File control `9`.
    - Pad with `9`×94 lines to a multiple of 10 records.
  - Unbalanced file; no offset debit (see Assumptions).
- `forms.rs`:
  - `pub fn form_941(q: &QuarterInput) -> Form941`.
    - `QuarterInput { year, quarter: u8, paychecks: Vec<PaycheckTotals { pay_date, period_start, period_end, employee_id: Uuid, fit_wages, fit, ss_wages, medicare_wages, addl_medicare_wages, ss_ee, ss_er, medicare_ee, medicare_er, addl_medicare }>, deposit_schedule: DepositSchedule { Monthly, Semiweekly } }`.
    - Output: line1 (distinct employees whose period contains the 12th of the quarter's 3rd month), line2, line3, line5a (wages, ×12.4%), line5c (×2.9%), line5d (×0.9%), line5e, line6, line7 (Σ actual SS/Medicare/addl ee+er − 5e), line10, line12, line16 (`MonthlyLiabilities([i64;3])` by pay-date month, or `ScheduleB(Vec<(NaiveDate, i64)>)` by pay date), and `total_liability`.
  - `pub fn form_940(y: &YearInput) -> Form940`: line3 total payments, line4 (Section 125), line5 (excess over $7,000 per employee), lines 6–8, line12, line16 quarterly FUTA liabilities, and `schedule_a_states: Vec<WorkState>` (non-empty iff more than one state).
  - `pub fn deposits(year, schedule, paychecks) -> Vec<Deposit { pay_date, liability_cents, due_date, next_day_rule: bool }>`.
    - Monthly: the 15th of the following month.
    - Semiweekly: paydays Wed/Thu/Fri close the period on Friday and are due the following Wednesday; paydays Sat–Tue close it on Tuesday and are due the following Friday. For each legal holiday among the 3 weekdays after the period closes, add one business day (Pub 15 "at least 3 business days").
    - Move forward past weekends and `legal_holidays`.
    - `next_day_rule` when a single day's liability ≥ 10,000,000.
- `w2.rs`: `pub fn w2s(year, employees: &[EmployeeYear]) -> Vec<W2>`. Fields:
  - box1 Σfit_wages, box2 Σfit
  - box3 min(Σss_wages, base), box4 Σss_ee
  - box5 Σmedicare_wages, box6 Σ(medicare_ee + addl)
  - box12: `D` Σ401k, `TT` Σqualified OT
  - box13 retirement plan = Σ401k > 0
  - no boxes 15–17 for WA/TX
- `efw2.rs`: `pub fn build(sub: &Submitter, employer: &EmployerIdentity, w2s: &[W2]) -> Result<String, Vec<Issue>>`, emitting RA, RE, one RW per employee, RT, and RF records of exactly 512 chars, separated by CRLF. Field positions **must be transcribed from SSA Pub 42-007 EFW2 Tax Year 2026** (https://www.ssa.gov/employer/efw/26efw2.pdf; the PDF blocks automated fetch, so download it in a browser). This covers the new 2026 RW field for code TT. Money fields are zero-filled right-justified cents in 11 positions. Validate SSN (9 digits, not 000/666/9xx area) and EIN (9 digits).
- `state_reports.rs`: `pub fn quarterly(kind: StateReport { WaEsdUi, WaPfml, WaLi, TxTwc }, rows: &[QuarterEmployeeRow]) -> StateReportOutput { header: Vec<&'static str>, rows: Vec<Vec<String>>, totals: Vec<(String, i64)> }`.
  - Columns:
    - WaEsdUi: `ssn,last_name,first_name,gross_wages,hours,soc_or_title`, with `hours = ceil(minutes/60)`. Totals: gross, excess over $78,200 YTD, taxable, tax.
    - WaPfml: `ssn,last_name,first_name,gross_wages,hours,wa_cares_exempt`. Totals: PFML total, ee, er, WA Cares.
    - WaLi: one row per class `class_code,worker_hours,employer_rate_tt,employee_rate_tt,premium_cents`, plus totals.
    - TxTwc: `ssn,last_name,first_name,gross_wages`. Totals: gross, taxable (≤ $9,000 YTD), tax.
  - These are data for portal entry, not agency bulk-file layouts (see Assumptions).
- Tests:
  - NACHA golden string for the two-entry file from Verification (entry hash `0003200003`, total credit `000000283384`, record count padded to 10).
  - 941 for the two paychecks: line2 337500, line3 25038, line5a 42470, line5c 9933, line5e 52403, line6 77441, line7 −1, line10 77440.
  - Deposit due dates: monthly Jan pay → 2026-02-17 (Feb 15 is a Sunday, Feb 16 a holiday); semiweekly Fri 2026-01-16 → Thu 2026-01-22 (nominal Wed 01-21 plus one day for MLK Day 01-19).
  - EFW2 record lengths are 512 and RW contains `00000137500`.

### 4. Shared AEAD sealing

- Create `crates/horae/src/sealing.rs` (`#[cfg(feature = "server")] mod sealing;` in `main.rs`). Move `cipher`, `encrypt`, `decrypt` and `decode_hex`, plus the `encrypt_then_decrypt_round_trips` and `nonce_makes_ciphertext_nondeterministic` tests, verbatim out of `importers/harvest/credentials.rs`. Use neutral messages: `"encryption key must be 32 bytes (64 hex chars)"` and `"decryption failed (wrong or rotated key)"`.
- In `credentials.rs`, import `crate::sealing::{encrypt, decrypt}`. Wrap `decrypt` in `load` with `.context("Harvest token decryption failed (key rotated? reconnect Harvest)")` so operator messaging is unchanged. Remove the moved functions; no re-exports.

### 5. Configuration and state

- `config.rs`:
  - Add `pub payroll: Option<PayrollConfig>` to `AppConfig`.
  - Add `pub struct PayrollConfig { pub encryption_key_hex: String }`, loaded as `non_empty("HORAE_PAYROLL_ENC_KEY").map(...)`. Validate 64 hex chars in `from_env` and fail startup with `"HORAE_PAYROLL_ENC_KEY must be 64 hex characters"`.
  - Field and parsing are compiled regardless of feature. In `main.rs` `Serve`, when `cfg.payroll.is_some()` and `!cfg!(feature = "payroll")`, log `tracing::warn!("HORAE_PAYROLL_ENC_KEY is set but this build does not include payroll")`.
- `state.rs`: add `pub payroll: Option<crate::config::PayrollConfig>`, `with_payroll(...)`, and a 7th `init_state` parameter. Update the `main.rs` call and the lazy `global_state` fallback (`.with_payroll(cfg.payroll)`).
- `.env.example` and the README env table: add `HORAE_PAYROLL_ENC_KEY` ("Enables payroll in builds with the `payroll` feature; 64 hex chars; seals SSNs and bank accounts").

### 6. Migration `crates/horae/migrations/0049_payroll.sql`

Unconditional: the tables exist (empty) in feature-off builds, which keeps one migration history and lets `reopen_period` (step 8) query them without cfg. Every table has `org_id uuid NOT NULL REFERENCES organizations(id)` and UUID v7 ids.

- `payroll_employers`:
  - `org_id` PK, `legal_name`, `ein char(9) CHECK (ein ~ '^[0-9]{9}$')`
  - `address_line1`, `address_line2`, `city`, `state char(2)`, `zip text`
  - `pay_frequency text CHECK IN ('weekly','biweekly')`, `period_anchor date`, `deposit_schedule text CHECK IN ('monthly','semiweekly')`
  - `overtime_weekly_threshold_minutes int NOT NULL DEFAULT 2400 CHECK (BETWEEN 1 AND 2400)`
  - `contact_name`, `contact_phone`, `contact_email`, `ssa_user_id text`
  - `odfi_routing char(9)`, `nacha_destination_name`, `nacha_company_name`, `nacha_company_identification char(10)`, `updated_at`
- `payroll_state_accounts`:
  - `(org_id, state, tax_year)` PK, `state text CHECK IN ('WA','TX')`
  - `ui_account text`, `ui_rate_bp int CHECK >= 0`
  - `pfml_employer_exempt bool NOT NULL DEFAULT false`, `li_account text`
- `payroll_li_risk_classes`: `(org_id, tax_year, class_code)` PK, `employer_rate_tt bigint`, `employee_rate_tt bigint`.
- `payroll_employees`:
  - `id` PK, `org_id`, `user_id` UNIQUE (FK `(user_id, org_id) REFERENCES users(id, org_id)`)
  - `legal_first_name`, `middle_initial`, `legal_last_name`, `suffix`
  - `ssn_enc bytea`, `ssn_last4 char(4)`
  - `address_line1`, `address_line2`, `city`, `state char(2)`, `zip`
  - `date_of_birth date`, `hire_date date NOT NULL`, `termination_date date`
  - `work_state text CHECK IN ('WA','TX')`, `flsa_exempt bool`, `wa_cares_exempt bool`
  - `li_class_code text`, `job_title_or_soc text`
  - `payment_method text CHECK IN ('direct_deposit','check')`, `bank_routing char(9)`, `bank_account_enc bytea`, `bank_account_type text CHECK IN ('checking','savings')`
  - `created_at`, `updated_at`
- `payroll_compensation`: `id`, `employee_id REFERENCES payroll_employees ON DELETE RESTRICT`, `effective_from date`, `basis text CHECK IN ('hourly','salary')`, `rate_cents bigint CHECK > 0`, `UNIQUE (employee_id, effective_from)`.
- `payroll_w4`: `id`, `employee_id`, `effective_from`, `form text CHECK IN ('2020_or_later','pre_2020')`, `filing_status text CHECK IN ('single','married_jointly','head_of_household')`, `multiple_jobs`, `dependents_cents`, `other_income_cents`, `deductions_cents`, `extra_withholding_cents`, `allowances int`, `exempt`, `UNIQUE (employee_id, effective_from)`.
- `payroll_deductions`: `id`, `employee_id`, `kind text CHECK IN ('retirement_401k','section125','post_tax')`, `description`, `amount_cents bigint CHECK > 0`, `starts_on date`, `ends_on date`.
- `payroll_runs`: `id`, `org_id`, `period_start`, `period_end`, `pay_date`, `status text CHECK IN ('draft','finalized')`, `created_by`, `created_at`, `finalized_by`, `finalized_at`, `UNIQUE (org_id, period_start)`.
- `payroll_paychecks`:
  - `id`, `org_id`, `run_id REFERENCES payroll_runs ON DELETE CASCADE`, `employee_id ... ON DELETE RESTRICT`, `user_id`
  - `pay_date`, `period_start`, `period_end`, `work_state`, `payment_method`
  - `regular_minutes`, `overtime_minutes`, `worked_minutes`, `sick_minutes`, `sick_accrued_minutes`
  - `gross_cents`, `net_cents`
  - `fit_wages_cents`, `ss_wages_cents`, `medicare_wages_cents`, `futa_wages_cents`, `suta_wages_cents`, `pfml_wages_cents`
  - `qualified_overtime_cents`, `li_class_code`
  - `UNIQUE (run_id, employee_id)`
- `payroll_paycheck_lines`: `id`, `paycheck_id ... ON DELETE CASCADE`, `org_id`, `code text`, `description`, `minutes int`, `rate_cents bigint`, `amount_cents bigint`, `employer bool`.
- Trigger `payroll_finalized_immutable`: `BEFORE UPDATE OR DELETE` on `payroll_runs` / `payroll_paychecks` / `payroll_paycheck_lines` raises when the owning run's status is `'finalized'`. Finalization itself updates `status` from draft, which is allowed.
- Run `horae-migrate` and regenerate `.sqlx` with `--features server,payroll` after steps 7–10 add their queries.

### 7. Host payroll module (`crates/horae/src/payroll.rs` + `payroll/`)

- In `main.rs`, add `#[cfg(feature = "server")] mod payroll;`. In `payroll.rs`:
  - `#[cfg(feature = "payroll")] pub(crate) mod api;`
  - `#[cfg(not(feature = "payroll"))] #[path = "payroll/unavailable.rs"] pub(crate) mod api;`
  - `#[cfg(feature = "payroll")] mod store; mod runs; mod filings; pub(crate) mod downloads;`
  - `unavailable.rs` mirrors every `api` fn signature. `status()` returns `Ok(PayrollStatus { included: false, configured: false })`; every other fn returns `Err(err(NOT_FOUND, "Payroll is not included in this build"))`. Clippy on both feature sets catches signature drift.
- `api.rs`: each fn starts with `let ctx = require_payroll().await?;`, which runs `require_admin()` and then reads `global_state().payroll`. A missing config gives `err(NOT_FOUND, "Payroll is not configured: set HORAE_PAYROLL_ENC_KEY")`. Map engine `Vec<Issue>` to DTO issues.
- `store.rs`: sqlx macros for the step-6 tables. SSN and account are sealed with `crate::sealing::encrypt(&cfg.encryption_key_hex, ...)`. Reads return `ssn_last4` only; decryption happens only in `filings.rs` (EFW2, state CSVs) and `runs.rs` (NACHA).
  - Saving an employee: upsert the employee row; upsert compensation/W-4 by `(employee_id, effective_from)`; replace the deduction list in one transaction.
  - `ssn: None` / `account: None` keep the stored ciphertext.
  - Validate: routing checksum, SSN 9 digits, `work_state == WA` ⇒ `li_class_code` and `job_title_or_soc` present. Errors use `BAD_REQUEST`.
- `runs.rs`:
  - `create(org, period_start, pay_date)`:
    - Load the employer and require it to exist.
    - `period_start` must be `period_anchor + k × (7 | 14)` days, else `BAD_REQUEST` "Pay periods start on {anchor weekday} every {n} days from {anchor}"; `period_end = start + 6 | 13`.
    - `INSERT payroll_runs status 'draft'` (`UNIQUE` → `CONFLICT` "A pay run for this period already exists"), then `calculate`.
  - `calculate(tx, run)`: delete the draft's paychecks, then for each employee employed during the period (`hire_date <= period_end AND (termination_date IS NULL OR termination_date >= period_start)`):
    - Time comes from `time_entries` with `spent_date` in the period, grouped by workweek (`week_start` = org `week_start` weekday ≤ date).
    - Issues:
      - entries in any week without an `approvals` row in state `'approved'` for `(user_id, week_start)` → "{name}: week of {date} is not approved"
      - any entry with state `open|submitted` → "{name}: week of {date} has unapproved entries"
    - Use **actual `minutes`**, not `rounded_minutes`.
    - Load compensation and W-4 effective on `period_start` and `pay_date` respectively (latest `effective_from <=`). No compensation → issue. Deductions active on `pay_date`.
    - Get the YTD sums from finalized paychecks with `pay_date` in the same calendar year (`suta_wages` filtered to the same `work_state`). Get the L&I rate from `payroll_li_risk_classes` for the year/class and `EmployerRates` from `payroll_state_accounts` (missing → issue "No {state} {year} state account").
    - Sick carryover = min(2400, previous-year balance) computed from finalized paychecks.
    - `employed_days` = days of the period within hire/termination.
    - Call `horae_payroll::compute_paycheck`. Store paycheck + lines; collect issues into an in-memory list returned with the detail. Issues are not persisted; drafts recompute.
  - `set_sick_minutes(run, user_id, minutes)`: draft only; stored in a draft-only column `sick_minutes` on the paycheck row, then recalculate.
  - `finalize(run)`: in one transaction:
    - `SELECT ... FROM payroll_runs WHERE id FOR UPDATE` (status must be draft, else `CONFLICT`).
    - `SELECT ... FROM approvals WHERE org_id AND user_id = ANY(...) AND period_start BETWEEN run.period_start AND run.period_end FOR SHARE`.
    - `calculate`. Any issue → rollback and `CONFLICT` with the issues joined by `"; "`.
    - `UPDATE status='finalized', finalized_by, finalized_at`.
  - `delete(run)`: draft only.
- `filings.rs`: aggregate finalized paychecks into the step-3 engine inputs per quarter/year/state. Sealed SSNs are decrypted here. New-hire rows are employees with `hire_date` in `[from, to]`.

### 8. Approvals: stable endpoints and the payroll lock

- `server_fns/approvals.rs`: replace `#[server]` with:
  - `#[dioxus_fullstack::post("/api/approvals/submit")]` on `submit_week`
  - `#[dioxus_fullstack::post("/api/approvals/approve")]` on `approve_submission`
  - `#[dioxus_fullstack::post("/api/approvals/reject")]` on `reject_submission`

  Signatures are unchanged (the client calls in `pages/approvals.rs` and `pages/timesheet.rs` keep working). Follow the existing pattern in `server_fns/importers.rs` (`start_harvest_api_import`).
- `server_fns/importers/authorization_tests.rs` `Api::call` `explicit` match: add `"submit_week" => Some("/api/approvals/submit")`, `"approve_submission" => Some("/api/approvals/approve")`, `"reject_submission" => Some("/api/approvals/reject")`. Callers: `own_submission.rs`, `time_entries/commands/tests/http.rs`. Extend `cli_job_endpoints_have_stable_registered_paths` with the three paths.
- In `reopen_period`, right after the `can_transition` check, add a SQL guard. When paid, return `conflict("This week is included in a finalized pay run and can no longer be reopened")`:
  ```sql
  SELECT EXISTS(SELECT 1 FROM payroll_paychecks pc JOIN payroll_runs r ON r.id = pc.run_id
    WHERE r.status = 'finalized' AND pc.org_id = $1 AND pc.user_id = $2
      AND pc.period_start <= $3 AND pc.period_end >= $4)
  ```
  The approval row is already `FOR UPDATE`-locked, which serializes against finalize's `FOR SHARE`. Add a `#[sqlx::test]` in `approvals.rs` `tests` that inserts a finalized run/paycheck covering a week and asserts the 409.

### 9. Server functions and DTOs

- `crates/horae/src/models/payroll.rs` (shared by client and server, no `horae-payroll` dependency; `pub mod payroll;` in `models.rs`). Mirror the engine enums with identical serde names: `WorkState`, `PayFrequency`, `DepositSchedule`, `PayBasis`, `FilingStatus`, `W4Form`, `DeductionKind`, `PaymentMethod { DirectDeposit, Check }`, `AccountType`. Structs:
  - `PayrollStatus { included: bool, configured: bool }`
  - `PayrollEmployer { legal_name, ein, address_line1, address_line2: Option<String>, city, state, zip, pay_frequency, period_anchor: NaiveDate, deposit_schedule, overtime_weekly_threshold_minutes: i32, contact_name, contact_phone, contact_email, ssa_user_id: Option<String>, odfi_routing, nacha_destination_name, nacha_company_name, nacha_company_identification }`
  - `StateAccountInput { state, tax_year: i32, ui_account, ui_rate_bp: i32, pfml_employer_exempt: bool, li_account: Option<String>, li_classes: Vec<LiRiskClass { class_code, employer_rate_tt: i64, employee_rate_tt: i64 }> }`
  - `PayrollEmployeeInput { user_id: Uuid, legal_first_name, middle_initial: Option<String>, legal_last_name, suffix: Option<String>, ssn: Option<String>, address_line1, address_line2: Option<String>, city, state, zip, date_of_birth: Option<NaiveDate>, hire_date: NaiveDate, termination_date: Option<NaiveDate>, work_state, flsa_exempt: bool, wa_cares_exempt: bool, li_class_code: Option<String>, job_title_or_soc: Option<String>, payment_method, bank: Option<BankAccountInput { routing, account: Option<String>, account_type }>, compensation: CompensationInput { effective_from, basis, rate_cents: i64 }, w4: Option<W4Input { effective_from, form, filing_status, multiple_jobs, dependents_cents, other_income_cents, deductions_cents, extra_withholding_cents, allowances: i32, exempt: bool }>, deductions: Vec<DeductionInput { kind, description, amount_cents, starts_on: NaiveDate, ends_on: Option<NaiveDate> }> }`
  - `PayrollEmployeeDetail` = the input with `ssn: None` plus `ssn_last4: Option<String>`, `bank_account_last4: Option<String>`
  - `PayRunSummary { id, period_start, period_end, pay_date, status: String, gross_cents, net_cents }`
  - `PayRunDetail { summary fields, issues: Vec<PayrollIssue { user_id: Option<Uuid>, message }>, paychecks: Vec<PaycheckView { id, user_id, employee_name, regular_minutes, overtime_minutes, sick_minutes, sick_accrued_minutes, sick_balance_minutes: Option<i32>, gross_cents, net_cents, lines: Vec<PaycheckLineView { code: String, description, minutes: Option<i32>, rate_cents: Option<i64>, amount_cents, employer: bool }> }> }`
- `crates/horae/src/server_fns/payroll.rs` (`mod payroll; pub use payroll::*;` in `server_fns.rs`). Every fn is `#[dioxus_fullstack::post(PATH)]` with a body that is one `crate::payroll::api::<fn>(...)` call:
  - `/api/payroll/status` `payroll_status()`
  - `/api/payroll/employer/get` `get_payroll_employer() -> Option<PayrollEmployer>`
  - `/api/payroll/employer/save` `save_payroll_employer(employer)`
  - `/api/payroll/state-accounts/list` `list_payroll_state_accounts()`
  - `/api/payroll/state-accounts/save` `save_payroll_state_account(account)`
  - `/api/payroll/employees/list` `list_payroll_employees()`
  - `/api/payroll/employees/get` `get_payroll_employee(user_id)`
  - `/api/payroll/employees/save` `save_payroll_employee(employee)`
  - `/api/payroll/runs/list` `list_pay_runs()`
  - `/api/payroll/runs/create` `create_pay_run(period_start, pay_date) -> PayRunDetail`
  - `/api/payroll/runs/get` `get_pay_run(run_id)`
  - `/api/payroll/runs/sick-leave` `set_pay_run_sick_minutes(run_id, user_id, sick_minutes)`
  - `/api/payroll/runs/finalize` `finalize_pay_run(run_id)`
  - `/api/payroll/runs/delete` `delete_pay_run(run_id)`

  Use the named status constants (`NOT_FOUND`, `CONFLICT`, `BAD_REQUEST`, `FORBIDDEN`) per the AGENTS.md rule.

### 10. Downloads, pay stubs and W-2 copies

- `payroll/downloads.rs`:
  - `pub fn router() -> axum::Router<...>`, merged in `main.rs` after `harvest::router` under `#[cfg(feature = "payroll")]`.
  - Auth helper `require_payroll_admin(&Session) -> Result<(Uuid, Uuid), StatusCode>`: copy `reports::require_manager`, but require `OrgRole::Admin` (401 no session, 403 non-admin, 404 payroll unconfigured).
  - Routes, all `GET`:
    - `/api/payroll/runs/{id}/register.csv` — one row per paycheck × line
    - `/api/payroll/runs/{id}/nacha.txt` — finalized only, else 409; direct-deposit paychecks only
    - `/api/payroll/runs/{id}/stubs.pdf` — finalized only
    - `/api/payroll/filings/{year}/{quarter}/941.csv`
    - `/api/payroll/filings/{year}/{quarter}/941-schedule-b.csv`
    - `/api/payroll/filings/{year}/940.csv`
    - `/api/payroll/filings/{year}/940-schedule-a.csv`
    - `/api/payroll/filings/{year}/w2.pdf`
    - `/api/payroll/filings/{year}/efw2.txt`
    - `/api/payroll/filings/{year}/{quarter}/state/{report}.csv` with `report ∈ wa-esd-ui | wa-pfml | wa-li | tx-twc`
    - `/api/payroll/filings/{year}/deposits.csv`
    - `/api/payroll/filings/new-hires.csv?from=&to=`
  - Form CSVs use the header `line,description,value` with value in dollars via `horae_core::money::format_cents_plain`. Use the `csv` crate, as `reports.rs` does.
- `render.rs`: change `build_engine()` to `build_engine(template: &'static str)`; the invoice call passes `INVOICE_TEMPLATE`.
  - Add `render_paystubs_pdf(stubs: &[PayStub]) -> anyhow::Result<Vec<u8>>` with `templates/paystub.typ`. One page per paycheck containing:
    - employer legal name and address; employee legal name and SSN last 4
    - pay period start/end and pay date (m/d/y)
    - workweek start day
    - hours: regular, overtime and sick, each with its rate
    - pay basis and rate
    - gross; every deduction and tax line; net; YTD per line
    - for WA non-exempt employees, sick leave accrued, used and balance
  - Add `render_w2_pdf(w2s) -> ...` with `templates/w2.typ`: employee Copies B, C and 2 with the official box numbers and captions per Pub 1141 employee-copy rules.

### 11. UI

Follow the `DESIGN.md` admin patterns. Use `pages::loaded`/`run_action`, one `#[component]` per file, and `use_resource` for data.

- `route.rs`: inside the `AdminShell` layout add
  - `#[route("/admin/payroll")] PayrollSettings {}`
  - `#[route("/admin/payroll/employees")] PayrollEmployees {}`
  - `#[route("/admin/payroll/runs")] PayRuns {}`
  - `#[route("/admin/payroll/runs/:id")] PayRunDetail { id: Uuid }`
  - `#[route("/admin/payroll/filings")] PayrollFilings {}`
- `pages/payroll.rs` + `pages/payroll/{settings,employees,runs,run_detail,filings}.rs`; add `pub mod payroll;` in `pages.rs`.
  - Settings: an employer form and per-state account forms with L&I classes.
  - Employees: a list of org users with payroll status, and an edit form for every `PayrollEmployeeInput` field. SSN/account fields are write-only, showing "•••• 1234".
  - Runs: a list plus a create form (period start, pay date).
  - Run detail: a paychecks table, issues alert, a sick-minutes input per WA non-exempt row (draft), and Finalize/Delete buttons. Once finalized, show download links for register, NACHA and stubs.
  - Filings: year/quarter pickers and the download links from step 10.
- `components/admin_shell.rs`:
  - Add `Section::Payroll` and `WorkspaceAccess.payroll`, with `payroll = legacy_admin && server_fns::payroll_status().await.map(|s| s.included && s.configured).unwrap_or(false)`.
  - Map the five routes to `Section::Payroll` in the `section` match, before the `_ => Section::Importers` fallback.
  - Add a "Payroll" group with links Settings, Employees, Pay runs and Filings.
  - Handle `Section::Payroll` in the `section == Section::Importers && own.is_err()` early return the same way as Importers.

### 12. NixOS test `nix/checks/e2e-payroll.nix`

- Copy the node block from `nix/checks/e2e.nix` (lines 4–40: TCG workarounds, `services.horae.enable`, `DEV_LOGIN=1`, `Restart = no`, package on PATH). Add `services.horae.secretKeyFile = pkgs.writeText "horae-payroll-env" "HORAE_PAYROLL_ENC_KEY=${64 × 'a'}";`. `name = "horae-e2e-payroll"`. Blueprint picks it up as `checks.<system>.e2e-payroll`.
- Copy the e2e boot sequence (console retry, `wait_for_unit`, `wait_for_open_port(3000)`) and the `sql()` helper. Add `api(path, body)`: `curl --fail-with-body -sS -b /tmp/cookies.txt -H 'content-type: application/json' --data-binary @- http://localhost:3000{path}`, with `json.dumps(body)` piped through `shlex.quote`, returning `json.loads`. Add `api_status(path, body)` returning the HTTP code via `-o /tmp/body -w '%{http_code}'` without `--fail`.
- Script:
  1. Run `horae seed`, then the dev-login cookie flow from `e2e.nix`. `ORG='01950000-0000-7000-8000-000000000001'`, `ADMIN='01950000-0000-7000-8000-000000000002'`.
  2. SQL: insert five `time_entries` for ADMIN, 2026-01-05..09, 540 min each, project `…0005`, task `…0007`, billable, state open. Insert user `TX='01950000-0000-7000-8000-0000000000f1'`, `tx@example.com`, `'Tex Salaried'`, role member.
  3. `api("/api/approvals/submit", {"week_start":"2026-01-05","context":{"expected_requester":{"org_id":ORG,"user_id":ADMIN},"subject_id":ADMIN,"expected_policy":"legacy_own"}})`, then `api("/api/approvals/approve", {"approval_id": id})`.
  4. `/api/payroll/employer/save`:
     - legal name "Demo Org LLC", EIN `123456789`
     - address "100 Pine St", Seattle, WA 98101
     - contact "Ada Admin", `2065550100`, `admin@example.com`; `ssa_user_id` `ABCD1234`
     - weekly, anchor `2026-01-05`, monthly deposits, threshold 2400
     - ODFI `121000248`, NACHA destination "WELLS FARGO", company name "DEMO ORG LLC", company identification `1123456789`
  5. `/api/payroll/state-accounts/save`: WA 2026 (`ui_rate_bp` 100, `li_classes` `[{"class_code":"4904-00","employer_rate_tt":10000,"employee_rate_tt":2000}]`) and TX 2026 (`ui_rate_bp` 270).
  6. `/api/payroll/employees/save` for ADMIN:
     - legal name "Ada Admin", address "1 Main St", Seattle, WA 98101, hire date 2026-01-01
     - WA, hourly 3000 effective 2026-01-01, non-exempt
     - W-4 `2020_or_later` single effective 2026-01-01, all amounts 0
     - 401(k) 5000 starting 2026-01-01
     - class `4904-00`, title "Developer"
     - SSN `223456789`, direct deposit `021000021`/`123456789` checking
  7. TX employee:
     - user TX, legal name "Tex Salaried", address "2 Elm St", Austin, TX 78701, hire date 2026-01-01
     - TX, salary 10400000 effective 2026-01-01, exempt
     - W-4 `2020_or_later` `married_jointly` with dependents 200000
     - SSN `234567890`, direct deposit `011000015`/`987654321` savings
  8. `run = api("/api/payroll/runs/create", {"period_start":"2026-01-05","pay_date":"2026-01-16"})`. Assert:
     - `run["issues"] == []`
     - ADMIN paycheck: `regular_minutes == 2400`, `overtime_minutes == 300`, `gross_cents == 142500`, `net_cents == 110453`, `fit` line 13269, `wa_li_ee` 900
     - TX paycheck: `net_cents == 172931`
  9. `api("/api/payroll/runs/finalize", {"run_id": run["id"]})` → `status == "finalized"`.
  10. NACHA download: every line is 94 chars, line count % 10 == 0, the `8` record contains `0003200003` and `000000283384`, and two `6` records with transaction codes `22` and `32`.
  11. `941.csv` for 2026 Q1: the line2/3/5a/5c/6/7/10 values from step 3, formatted `3375.00`, `250.38`, `424.70`, `99.33`, `774.41`, `-0.01`, `774.40`.
  12. `efw2.txt`: each record is 512 chars, two RW records, and the ADMIN RW contains `223456789`, `00000137500` and `00000007500`.
  13. `stubs.pdf` starts with `%PDF`.
  14. `api_status("/api/approvals/reject", {"approval_id": id}) == "409"`, and the body contains "finalized pay run". SQL confirms the approval row still exists.
  15. Anonymous `curl` of `nacha.txt` returns 401.

## Critical files & anchors

- `crates/horae/src/server_fns/approvals.rs` — `reopen_period` (lock guard) and the three endpoint attributes.
- `crates/horae/src/server_fns/importers/authorization_tests.rs` — `Api::call` explicit-path map and `cli_job_endpoints_have_stable_registered_paths`.
- `crates/horae/src/importers/harvest/credentials.rs` — `cipher`/`encrypt`/`decrypt`/`decode_hex` move to `sealing.rs`.
- `crates/horae/src/components/admin_shell.rs` — `Section` match fallback `_ => Section::Importers` must not swallow the payroll routes.
- `nix/package.nix` `dxBuild` and `checkArtifacts` — the feature must reach the server build and the check artifacts.

## Verification

Prerequisite: `nix develop`, `process-compose up postgres migrate`, `DATABASE_URL=postgres://horae@127.0.0.1:5432/horae`.

1. Engine:
   - `cargo test -p horae-payroll`: WA/TX golden paychecks (net 110453 / 172931), the caps, issue messages, NACHA golden, 941 lines, deposit dates and EFW2 lengths all pass.
   - `cargo check -p horae-payroll --target wasm32-unknown-unknown` succeeds.
2. Host:
   - `cargo test -p horae --features server,payroll`: the new reopen-lock `#[sqlx::test]` returns 409, and the existing approval and authorization suites pass with the stable paths.
   - `cargo clippy -p horae --features server --all-targets -- -D warnings` (feature off) and the same with `server,payroll` both pass.
3. Cache:
   - Run `touch crates/horae/src/main.rs crates/horae/tests/*.rs`, then `cargo sqlx prepare --workspace -- --features server,payroll --all-targets`.
   - Then `SQLX_OFFLINE=true cargo test -p horae --features server,payroll --no-run` builds.
4. Manual UI smoke: `process-compose up`, then http://localhost:8080/auth/login → "Sign in as Admin" → Admin → Payroll.
   - Fill in settings and an employee.
   - Submit and approve a week on /timesheet and /approvals.
   - Create a run, check the figures, finalize, and download the stubs PDF and NACHA file.
5. End to end: `nix build .#checks.x86_64-linux.e2e-payroll` passes every assertion in step 12. Also run `nix build .#checks.x86_64-linux.{tests,clippy,sqlx-prepare,e2e}` to confirm the existing checks still pass with the feature-enabled package.

## Assumptions & contingencies

- **Corrections after finalization are out of scope.** Finalized runs are immutable, and reopening a paid week is refused. Retro back-pay or off-cycle runs and W-2c are not built. If a correction is needed, the operator handles it outside Horae.
- **Salaried employees must be FLSA-exempt.** Non-exempt staff are paid hourly; this avoids fluctuating-workweek regular-rate rules. No bonuses, tips, PTO pay (other than WA sick leave), or supplemental-wage withholding.
- **One work state per employee** (WA or TX); no multi-state allocation within a paycheck. Local taxes don't apply to WA/TX.
- **Pay frequency is weekly or biweekly only**, so pay periods align with Horae's approval workweeks and with the TX semimonthly-minimum and WA monthly-minimum payday rules.
- **State reports are portal-entry data (CSV)**, not agency bulk-upload layouts. Federal EFW2 is the only e-file format generated. Nothing is transmitted.
- **NACHA files are unbalanced** (credits only, service class 220). If the operator's bank requires balanced files, add an offset debit entry (transaction code 27, service class 200) from a sealed employer account in `payroll_employers`.
- **Runtime enablement follows the Harvest pattern.** `HORAE_PAYROLL_ENC_KEY` in `services.horae.secretKeyFile` enables payroll; there is no new NixOS option. The shipped package always includes the `payroll` feature.
- **The SUTA wage base for TX excludes Section 125, and WA includes it.** The WA rule follows the ESD handbook; the TX rule is unverified. If TWC guidance differs, change only `state/tx.rs` step 4.
- **WA PFML/WA Cares wages include 401(k) and Section 125 deferrals** ("gross wages" per the ESD toolkit). If ESD clarifies otherwise, change only `state/wa.rs`.
- **FUTA credit-reduction rates are 0 for WA/TX in 2026.** DOL publishes the 2026 list in November 2026. If WA or TX appears, set `futa_credit_reduction_bp` in `year/y2026.rs`.
- **EFW2 positions:** if the 2026 Pub 42-007 cannot be downloaded during implementation, stop and ask the user for the PDF. Do not reuse earlier-year layouts; 2026 changed the record layout.
- **Rounding** is half away from zero to the cent at every published worksheet step. Pub 15-T allows whole-dollar rounding, which this plan does not use.
