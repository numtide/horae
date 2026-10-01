// Values are literal text inputs, never evaluated as Typst source.
#import sys: inputs
#set document(date: none, title: "Project summary — " + inputs.project)
#set page(paper: "a4", margin: 2cm, numbering: "1")
#set text(size: 10pt)

= Project work summary
#text(weight: "bold", inputs.project) \
#text(inputs.client) \
#text(inputs.period)

#v(12pt)
#text("Total hours: " + inputs.total.hours + " (" + inputs.total.minutes + " minutes)") \
#text("Billable: " + inputs.billable + " · Non-billable: " + inputs.non_billable) \
#text("Internal costs: " + inputs.total.cost)

Tracked minutes, without invoice rounding. Costs use workspace currency and
current permitted cost rates; missing rates are not treated as zero.
This selected-period work summary does not include lifetime budgets or invoices.

#let breakdown(title, rows) = [
  == #title
  #if rows.len() == 0 [No contributors in this project.]
  #table(
    columns: (1fr, auto, auto, auto),
    inset: 6pt,
    stroke: 0.4pt + luma(200),
    table.header([Name], [Hours], [Minutes], [Internal costs]),
    ..rows.map(row => (text(row.name), text(row.hours), text(row.minutes), text(row.cost))).flatten(),
    [Total], text(inputs.total.hours), text(inputs.total.minutes), text(inputs.total.cost),
  )
]

#breakdown("Tasks", inputs.tasks)
#breakdown("Team", inputs.people)
