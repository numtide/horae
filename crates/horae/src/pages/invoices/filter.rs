//! The Invoices list's URL query: `?tab=uninvoiced&status=overdue&client=<id>&period=last-year&q=<search>`.
//! It uses only `std` and `uuid`, so the route mirrors in `tests/` can include
//! this exact file.

use std::fmt;

use uuid::Uuid;

/// The status filter. Overdue is computed, not stored; Sent includes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusValue {
    Draft,
    Sent,
    Overdue,
    Paid,
    Void,
    /// Written off; only invoices imported from Harvest reach it.
    Closed,
}

impl StatusValue {
    pub const ALL: [Self; 6] = [
        Self::Draft,
        Self::Sent,
        Self::Overdue,
        Self::Paid,
        Self::Void,
        Self::Closed,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Sent => "sent",
            Self::Overdue => "overdue",
            Self::Paid => "paid",
            Self::Void => "void",
            Self::Closed => "written-off",
        }
    }
}

/// The period filter, over the issue date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodValue {
    ThisMonth,
    LastMonth,
    ThisYear,
    LastYear,
}

impl PeriodValue {
    pub const ALL: [Self; 4] = [
        Self::ThisMonth,
        Self::LastMonth,
        Self::ThisYear,
        Self::LastYear,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ThisMonth => "this-month",
            Self::LastMonth => "last-month",
            Self::ThisYear => "this-year",
            Self::LastYear => "last-year",
        }
    }
}

/// What the Invoices list shows, all of it in the URL so a view can be
/// shared and Back returns to it. The default is every invoice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InvoiceFilter {
    pub uninvoiced: bool,
    pub status: Option<StatusValue>,
    pub client: Option<Uuid>,
    pub period: Option<PeriodValue>,
    pub search: String,
}

impl InvoiceFilter {
    /// The same view with no status, client, period or search.
    pub fn cleared(&self) -> Self {
        Self {
            uninvoiced: self.uninvoiced,
            ..Self::default()
        }
    }
}

/// Reads the query leniently: a value it does not know is left out. The
/// router decodes the query once before this, so the search, written last,
/// is everything after `q=`, `&` and `=` included.
impl From<&str> for InvoiceFilter {
    fn from(query: &str) -> Self {
        let mut filter = Self::default();
        let mut rest = query;
        while !rest.is_empty() {
            if let Some(search) = rest.strip_prefix("q=") {
                filter.search = search.to_owned();
                break;
            }
            let (pair, tail) = rest.split_once('&').unwrap_or((rest, ""));
            rest = tail;
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            match key {
                "tab" => filter.uninvoiced = value == "uninvoiced",
                "status" => {
                    filter.status = StatusValue::ALL.into_iter().find(|s| s.as_str() == value)
                }
                "client" => filter.client = Uuid::parse_str(value).ok(),
                "period" => {
                    filter.period = PeriodValue::ALL.into_iter().find(|p| p.as_str() == value)
                }
                _ => {}
            }
        }
        filter
    }
}

impl fmt::Display for InvoiceFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.uninvoiced {
            parts.push("tab=uninvoiced".to_owned());
        }
        if let Some(status) = self.status {
            parts.push(format!("status={}", status.as_str()));
        }
        if let Some(client) = self.client {
            parts.push(format!("client={client}"));
        }
        if let Some(period) = self.period {
            parts.push(format!("period={}", period.as_str()));
        }
        if !self.search.is_empty() {
            // The router encodes spaces, `#` and the rest it must, and
            // decodes the whole query once before `from`, but leaves `%`
            // alone, so a literal `%` is escaped here.
            parts.push(format!("q={}", self.search.replace('%', "%25")));
        }
        f.write_str(&parts.join("&"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the router hands `from`: the query decoded once.
    fn decoded(written: &str) -> String {
        let mut out = String::new();
        let mut chars = written.chars();
        while let Some(c) = chars.next() {
            if c == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                out.push(u8::from_str_radix(&hex, 16).unwrap() as char);
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn every_filter_survives_the_url() {
        let filter = InvoiceFilter {
            uninvoiced: false,
            status: Some(StatusValue::Overdue),
            client: Some(Uuid::now_v7()),
            period: Some(PeriodValue::LastYear),
            search: "A&B = 100% #2".into(),
        };
        let written = filter.to_string();
        assert_eq!(InvoiceFilter::from(decoded(&written).as_str()), filter);
    }

    #[test]
    fn the_default_writes_no_query_and_unknown_values_are_dropped() {
        assert_eq!(InvoiceFilter::default().to_string(), "");
        let filter = InvoiceFilter::from("tab=uninvoiced&status=lost&client=nope&period=forever");
        assert_eq!(
            filter,
            InvoiceFilter {
                uninvoiced: true,
                ..Default::default()
            }
        );
    }
}
