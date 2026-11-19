"""The quickstart scenarios of spec 022 (US1, US3, US6), run with a client
generated from the server's own /api/v1/openapi.json and used unmodified.

Reads HORAE_API_BASE (…/api/v1) and HORAE_ACCESS_TOKEN; exits non-zero on the
first failed check."""

import json
import os
from datetime import date, datetime, timedelta, timezone

import horae_client
from horae_client import NewTimeEntry, NewTimer, TimeEntryPatch
from horae_client.exceptions import ApiException

BASE = os.environ["HORAE_API_BASE"]
TOKEN = os.environ["HORAE_ACCESS_TOKEN"]


def client(token):
    return horae_client.ApiClient(horae_client.Configuration(host=BASE, access_token=token))


api = client(TOKEN)
identity = horae_client.IdentityApi(api)
catalogue = horae_client.CatalogueApi(api)
entries = horae_client.TimeEntriesApi(api)
timers = horae_client.TimersApi(api)


def step(name):
    print(f"-- {name}", flush=True)


def problem(call, status, code):
    """`call` fails with `status`, answered as problem+json with `code`."""
    try:
        call()
    except ApiException as error:
        assert error.status == status, (error.status, error.body)
        assert error.headers["Content-Type"] == "application/problem+json", error.headers
        body = json.loads(error.body)
        assert body["status"] == status and body["code"] == code, body
        return body
    raise AssertionError(f"expected {status} {code}")


def walk(list_page, **filters):
    """Every item of a list, page by page through `next_cursor`."""
    items, cursor = [], None
    while True:
        page = list_page(limit=2, cursor=cursor, **filters)
        items += page.data
        cursor = page.next_cursor
        if cursor is None:
            return items


step("US1: the token reads as its owner")
me = identity.get_me()
assert me.person.email == "admin@example.com", me
assert me.administrator and me.token.name == "seed", me
problem(lambda: horae_client.IdentityApi(client("horae_pat_unknown")).get_me(), 401, "unauthenticated")

step("US1: the catalogue")
projects = walk(catalogue.list_projects)
tasks = walk(catalogue.list_tasks)
clients = walk(catalogue.list_clients)
people = walk(catalogue.list_people)
assert projects and tasks and clients and people
assert catalogue.get_project(projects[0].id).id == projects[0].id
assert any(person.id == me.person.id for person in people)

step("US6: a cursor walk returns each entry once")
started = datetime.now(timezone.utc)
seeded = walk(entries.list_time_entries)
ids = [entry.id for entry in seeded]
assert ids and len(ids) == len(set(ids)) and ids == sorted(ids, key=str), ids
everything = entries.list_time_entries(limit=1000)
assert [entry.id for entry in everything.data] == ids and everything.next_cursor is None
page = entries.list_time_entries(limit=1)
problem(lambda: entries.list_time_entries(limit=1, cursor=page.next_cursor, is_running=True), 400, "invalid_cursor")

step("US3: create and edit an entry")
pair = identity.list_trackable().data[0]
yesterday = date.today() - timedelta(days=1)
created = entries.create_time_entry(
    NewTimeEntry(project_id=pair.project_id, task_id=pair.task_id, spent_date=yesterday, minutes=90, notes="API")
)
assert created.minutes == 90 and created.notes == "API" and created.state == "open", created
patched = entries.update_time_entry(created.id, TimeEntryPatch(minutes=45))
assert patched.minutes == 45 and patched.notes == "API" and patched.spent_date == yesterday, patched
assert entries.get_time_entry(created.id) == patched

step("US6: updated_since returns what changed after the walk began")
changed = entries.list_time_entries(updated_since=started).data
assert [entry.id for entry in changed] == [created.id], changed

step("US3: start, stop and restart a timer")
running = timers.start_timer(NewTimer(project_id=pair.project_id, task_id=pair.task_id))
assert running.is_running, running
assert identity.get_running_timer().id == running.id
stopped = timers.stop_timer(running.id)
assert not stopped.is_running, stopped
problem(lambda: timers.stop_timer(running.id), 409, "conflict")
restarted = timers.restart_timer(running.id)
assert restarted.is_running and restarted.id == running.id, restarted
second = timers.start_timer(NewTimer(project_id=pair.project_id, task_id=pair.task_id))
assert not entries.get_time_entry(running.id).is_running, "starting a timer stops the running one"
timers.stop_timer(second.id)

step("US3: refusals are problem+json with the web app's reason")
problem(lambda: timers.restart_timer(created.id), 409, "conflict")
problem(lambda: entries.get_time_entry("00000000-0000-7000-8000-000000000000"), 404, "not_found")
problem(lambda: entries.list_time_entries(var_from=yesterday, to=yesterday - timedelta(days=1)), 422, "invalid")

step("US3: delete")
entries.delete_time_entry(created.id)
problem(lambda: entries.get_time_entry(created.id), 404, "not_found")

print("api-v1: every scenario passed")
