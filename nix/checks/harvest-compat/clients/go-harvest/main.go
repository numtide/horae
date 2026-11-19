// Read workflow through github.com/becoded/go-harvest, unmodified, against a
// Horae server: who am I, the project picker, and the time entries with one
// fetched by id. Typed decoding is the point: integer ids and numeric hours
// must parse into the library's own structs.
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
}
