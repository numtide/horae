// Read and write workflows through github.com/becoded/go-harvest, unmodified,
// against a Horae server: who am I, the project picker, the time entries with
// one fetched by id, the company and the person's project assignments, then
// the time workflow (log, edit, start, stop, restart, delete). Typed decoding
// is the point: integer ids and numeric hours must parse into the library's
// own structs.
//
// The client authenticates as a Harvest personal access token does: the token
// as a bearer credential and the account id in Harvest-Account-Id.
package main

import (
	"context"
	"fmt"
	"net/http"
	"net/url"
	"os"
	"time"

	"github.com/becoded/go-harvest/harvest"
)

type tokenTransport struct{ token, account string }

func (t tokenTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	req = req.Clone(req.Context())
	req.Header.Set("Authorization", "Bearer "+t.token)
	req.Header.Set("Harvest-Account-Id", t.account)
	return http.DefaultTransport.RoundTrip(req)
}

func check(err error, step string) {
	if err != nil {
		fmt.Fprintf(os.Stderr, "%s: %v\n", step, err)
		os.Exit(1)
	}
}

func main() {
	base, err := url.Parse(os.Getenv("HORAE_API_BASE"))
	check(err, "base url")
	client := harvest.NewAPIClient(&http.Client{
		Transport: tokenTransport{
			token:   os.Getenv("HORAE_ACCESS_TOKEN"),
			account: os.Getenv("HORAE_ACCOUNT_ID"),
		},
	})
	client.BaseURL = base
	ctx := context.Background()

	me, _, err := client.User.Current(ctx)
	check(err, "users/me")
	fmt.Printf("users/me: id=%d\n", *me.ID)

	projects, _, err := client.Project.List(ctx, &harvest.ProjectListOptions{})
	check(err, "projects")
	if len(projects.Projects) == 0 {
		check(fmt.Errorf("no projects"), "projects")
	}
	fmt.Printf("projects: %d\n", len(projects.Projects))

	entries, _, err := client.Timesheet.List(ctx, &harvest.TimeEntryListOptions{})
	check(err, "time_entries")
	if len(entries.TimeEntries) == 0 {
		check(fmt.Errorf("no time entries"), "time_entries")
	}
	first := entries.TimeEntries[0]
	entry, _, err := client.Timesheet.Get(ctx, *first.ID)
	check(err, "time_entries/{id}")
	if *entry.Hours != *first.Hours {
		check(fmt.Errorf("hours %v != %v", *entry.Hours, *first.Hours), "time_entries/{id}")
	}
	fmt.Printf("time_entries: %d, first id=%d hours=%v\n", len(entries.TimeEntries), *entry.ID, *entry.Hours)

	company, _, err := client.Company.Get(ctx)
	check(err, "company")
	if company.Name == nil || company.WeekStartDay == nil {
		check(fmt.Errorf("no name or week start: %v", company), "company")
	}
	fmt.Printf("company: %s, week starts %s\n", *company.Name, *company.WeekStartDay)

	assignments, _, err := client.User.GetMyProjectAssignments(ctx, &harvest.MyProjectAssignmentListOptions{})
	check(err, "users/me/project_assignments")
	if len(assignments.ProjectAssignments) == 0 {
		check(fmt.Errorf("no assignments"), "users/me/project_assignments")
	}
	picked := assignments.ProjectAssignments[0]
	if picked.TaskAssignments == nil || len(*picked.TaskAssignments) == 0 {
		check(fmt.Errorf("no task assignments in %v", picked), "users/me/project_assignments")
	}
	project, task := *picked.Project.ID, *(*picked.TaskAssignments)[0].Task.ID
	fmt.Printf("project_assignments: %d, tracking project=%d task=%d\n",
		len(assignments.ProjectAssignments), project, task)

	writes(ctx, client, project, task)
	sync(ctx, client)
}

// Time workflow through the library's write calls: log a duration, edit it,
// log a start and end time, run a timer that a second timer stops (D-6),
// stop and restart, then delete everything it created.
func writes(ctx context.Context, client *harvest.APIClient, project, task int64) {
	today := harvest.Date{Time: time.Now()}
	expect := func(entry *harvest.TimeEntry, hours float64, running bool, step string) {
		if *entry.Hours != hours || *entry.IsRunning != running {
			check(fmt.Errorf("hours=%v running=%v, want %v and %v",
				*entry.Hours, *entry.IsRunning, hours, running), step)
		}
	}
	var created []int64

	hours, notes := 1.5, "Logged by go-harvest"
	logged, _, err := client.Timesheet.CreateTimeEntryViaDuration(ctx, &harvest.TimeEntryCreateViaDuration{
		ProjectID: &project, TaskID: &task, SpentDate: &today, Hours: &hours, Notes: &notes,
	})
	check(err, "create via duration")
	expect(logged, 1.5, false, "create via duration")
	created = append(created, *logged.ID)

	quarter, edited := 0.25, "Edited by go-harvest"
	updated, _, err := client.Timesheet.UpdateTimeEntry(ctx, *logged.ID, &harvest.TimeEntryUpdate{
		ProjectID: &project, TaskID: &task, SpentDate: &today, Hours: &quarter, Notes: &edited,
	})
	check(err, "update")
	expect(updated, 0.25, false, "update")
	if *updated.Notes != edited {
		check(fmt.Errorf("notes %q", *updated.Notes), "update")
	}

	start := harvest.Time{Time: time.Date(2000, 1, 1, 9, 0, 0, 0, time.Local)}
	end := harvest.Time{Time: time.Date(2000, 1, 1, 10, 30, 0, 0, time.Local)}
	span, _, err := client.Timesheet.CreateTimeEntryViaStartEndTime(ctx, &harvest.TimeEntryCreateViaStartEndTime{
		ProjectID: &project, TaskID: &task, SpentDate: &today, StartedTime: &start, EndedTime: &end,
	})
	check(err, "create via start and end time")
	expect(span, 1.5, false, "create via start and end time")
	if span.StartedTime == nil || span.StartedTime.String() != "9:00am" ||
		span.EndedTime == nil || span.EndedTime.String() != "10:30am" {
		check(fmt.Errorf("times %v to %v", span.StartedTime, span.EndedTime), "create via start and end time")
	}
	created = append(created, *span.ID)

	timer := &harvest.TimeEntryCreateViaDuration{ProjectID: &project, TaskID: &task, SpentDate: &today}
	first, _, err := client.Timesheet.CreateTimeEntryViaDuration(ctx, timer)
	check(err, "start timer")
	expect(first, 0, true, "start timer")
	created = append(created, *first.ID)
	second, _, err := client.Timesheet.CreateTimeEntryViaDuration(ctx, timer)
	check(err, "start second timer")
	created = append(created, *second.ID)
	first, _, err = client.Timesheet.Get(ctx, *first.ID)
	check(err, "first timer after the second started")
	if *first.IsRunning {
		check(fmt.Errorf("still running"), "first timer after the second started")
	}

	stopped, _, err := client.Timesheet.StopTimeEntry(ctx, *second.ID)
	check(err, "stop")
	if *stopped.IsRunning {
		check(fmt.Errorf("still running"), "stop")
	}
	restarted, _, err := client.Timesheet.RestartTimeEntry(ctx, *first.ID)
	check(err, "restart")
	if !*restarted.IsRunning {
		check(fmt.Errorf("not running"), "restart")
	}
	_, _, err = client.Timesheet.StopTimeEntry(ctx, *first.ID)
	check(err, "stop restarted")

	for _, id := range created {
		_, err := client.Timesheet.DeleteTimeEntry(ctx, id)
		check(err, fmt.Sprintf("delete %d", id))
	}
	if _, _, err := client.Timesheet.Get(ctx, created[0]); err == nil {
		check(fmt.Errorf("entry %d still readable", created[0]), "delete")
	}
	fmt.Printf("writes: %d entries created, edited, timed and deleted\n", len(created))
}
