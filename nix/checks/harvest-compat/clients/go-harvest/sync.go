package main

import (
	"context"
	"fmt"
	"net/http"
	"slices"
	"time"

	"github.com/becoded/go-harvest/harvest"
)

// A page of one list, decoded into the library's own list type.
type listing struct {
	ids        []int64
	pagination harvest.Pagination
}

// A list as the sync workflow walks it: by page number through the library's
// List call, and by link through the library's request and decoding.
type syncedList struct {
	name   string
	byPage func(page int) (listing, error)
	byLink func(link string) (listing, error)
	// Changed since a time, through the library's updated_since option.
	since func(mark time.Time) (listing, error)
}

const syncPerPage = 3

func ids[T any](items []*T, id func(*T) *int64) []int64 {
	out := make([]int64, 0, len(items))
	for _, item := range items {
		out = append(out, *id(item))
	}
	return out
}

// follow fetches an absolute link the server handed out and decodes it.
func follow[L any](ctx context.Context, client *harvest.APIClient, link string) (*L, error) {
	var list L
	req, err := client.NewRequest(ctx, http.MethodGet, link, nil)
	if err != nil {
		return &list, err
	}
	_, err = client.Do(ctx, req, &list)
	return &list, err
}

func syncedLists(ctx context.Context, client *harvest.APIClient) []syncedList {
	page := func(n int) harvest.ListOptions { return harvest.ListOptions{Page: n, PerPage: syncPerPage} }
	timeEntries := func(l *harvest.TimeEntryList) listing {
		return listing{ids(l.TimeEntries, func(e *harvest.TimeEntry) *int64 { return e.ID }), l.Pagination}
	}
	clients := func(l *harvest.ClientList) listing {
		return listing{ids(l.Clients, func(c *harvest.Client) *int64 { return c.ID }), l.Pagination}
	}
	projects := func(l *harvest.ProjectList) listing {
		return listing{ids(l.Projects, func(p *harvest.Project) *int64 { return p.ID }), l.Pagination}
	}
	tasks := func(l *harvest.TaskList) listing {
		return listing{ids(l.Tasks, func(t *harvest.Task) *int64 { return t.ID }), l.Pagination}
	}
	users := func(l *harvest.UserList) listing {
		return listing{ids(l.Users, func(u *harvest.User) *int64 { return u.ID }), l.Pagination}
	}
	return []syncedList{
		{
			name: "time_entries",
			byPage: func(n int) (listing, error) {
				l, _, err := client.Timesheet.List(ctx, &harvest.TimeEntryListOptions{ListOptions: page(n)})
				return timeEntries(l), err
			},
			byLink: func(link string) (listing, error) {
				l, err := follow[harvest.TimeEntryList](ctx, client, link)
				return timeEntries(l), err
			},
			since: func(mark time.Time) (listing, error) {
				l, _, err := client.Timesheet.List(ctx, &harvest.TimeEntryListOptions{UpdatedSince: &mark})
				return timeEntries(l), err
			},
		},
		{
			name: "clients",
			byPage: func(n int) (listing, error) {
				l, _, err := client.Client.List(ctx, &harvest.ClientListOptions{ListOptions: page(n)})
				return clients(l), err
			},
			byLink: func(link string) (listing, error) {
				l, err := follow[harvest.ClientList](ctx, client, link)
				return clients(l), err
			},
			since: func(mark time.Time) (listing, error) {
				l, _, err := client.Client.List(ctx, &harvest.ClientListOptions{UpdatedSince: mark})
				return clients(l), err
			},
		},
		{
			name: "projects",
			byPage: func(n int) (listing, error) {
				l, _, err := client.Project.List(ctx, &harvest.ProjectListOptions{ListOptions: page(n)})
				return projects(l), err
			},
			byLink: func(link string) (listing, error) {
				l, err := follow[harvest.ProjectList](ctx, client, link)
				return projects(l), err
			},
			since: func(mark time.Time) (listing, error) {
				l, _, err := client.Project.List(ctx, &harvest.ProjectListOptions{UpdatedSince: mark})
				return projects(l), err
			},
		},
		{
			name: "tasks",
			byPage: func(n int) (listing, error) {
				l, _, err := client.Task.List(ctx, &harvest.TaskListOptions{ListOptions: page(n)})
				return tasks(l), err
			},
			byLink: func(link string) (listing, error) {
				l, err := follow[harvest.TaskList](ctx, client, link)
				return tasks(l), err
			},
			since: func(mark time.Time) (listing, error) {
				l, _, err := client.Task.List(ctx, &harvest.TaskListOptions{UpdatedSince: mark})
				return tasks(l), err
			},
		},
		{
			name: "users",
			byPage: func(n int) (listing, error) {
				l, _, err := client.User.List(ctx, &harvest.UserListOptions{ListOptions: page(n)})
				return users(l), err
			},
			byLink: func(link string) (listing, error) {
				l, err := follow[harvest.UserList](ctx, client, link)
				return users(l), err
			},
			since: func(mark time.Time) (listing, error) {
				l, _, err := client.User.List(ctx, &harvest.UserListOptions{UpdatedSince: mark})
				return users(l), err
			},
		},
	}
}

// Sync workflow (spec 021 US3): a page-counting client and a link-following
// client each pull every list in pages of three and must see the same records,
// each once, as many as the list reports. Then one time entry is edited and an
// incremental run since just before the edit sees that entry and nothing else.
func sync(ctx context.Context, client *harvest.APIClient) {
	lists := syncedLists(ctx, client)
	for _, list := range lists {
		first, err := list.byPage(1)
		check(err, list.name+" page 1")
		byPage := first.ids
		for n := 2; n <= *first.pagination.TotalPages; n++ {
			next, err := list.byPage(n)
			check(err, fmt.Sprintf("%s page %d", list.name, n))
			byPage = append(byPage, next.ids...)
		}

		byLink := first.ids
		for current := first; current.pagination.Links.Next != nil; {
			current, err = list.byLink(*current.pagination.Links.Next)
			check(err, list.name+" next link")
			byLink = append(byLink, current.ids...)
		}

		total := *first.pagination.TotalEntries
		unique := slices.Compact(slices.Sorted(slices.Values(byPage)))
		switch {
		case len(byPage) != total || len(unique) != total:
			check(fmt.Errorf("%d records, %d distinct, of %d", len(byPage), len(unique), total), list.name+" page walk")
		case !slices.Equal(byPage, byLink):
			check(fmt.Errorf("links gave %v, pages %v", byLink, byPage), list.name+" link walk")
		}
		fmt.Printf("sync %s: %d records in %d pages, by page and by link\n",
			list.name, total, *first.pagination.TotalPages)
	}

	// updated_since goes out in whole seconds: start the mark on a fresh one.
	mark := time.Now().Truncate(time.Second).Add(time.Second)
	time.Sleep(time.Until(mark))
	entries, err := lists[0].byPage(1)
	check(err, "time_entries before the edit")
	edited, notes := entries.ids[0], "Edited for the incremental sync"
	_, _, err = client.Timesheet.UpdateTimeEntry(ctx, edited, &harvest.TimeEntryUpdate{Notes: &notes})
	check(err, "edit for the incremental sync")
	for _, list := range lists {
		changed, err := list.since(mark)
		check(err, list.name+" updated_since")
		want := []int64{}
		if list.name == "time_entries" {
			want = []int64{edited}
		}
		if !slices.Equal(changed.ids, want) {
			check(fmt.Errorf("got %v, want %v", changed.ids, want), list.name+" updated_since")
		}
	}
	fmt.Printf("sync incremental: only time entry %d changed\n", edited)
}
