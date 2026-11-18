use super::*;

const KEY: [u8; 32] = [7; 32];

fn key() -> CursorKey {
    CursorKey::new(KEY)
}

fn walk<'a>(key: &'a CursorKey, list: &'a str, filters: &'a str) -> Walk<'a> {
    Walk { key, list, filters }
}

fn now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_800_000_000, 0).unwrap()
}

#[test]
fn a_cursor_round_trips() {
    let key = key();
    let walk = walk(&key, "time_entries", r#"{"person_id":null}"#);
    let id = Uuid::now_v7();
    let cursor = walk.encode(id, now());
    assert_eq!(walk.decode(&cursor, now()), Some(id));
    assert_eq!(walk.decode(&cursor, now() + MAX_AGE), Some(id));
    assert!(
        cursor
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "{cursor}"
    );
}

#[test]
fn a_tampered_byte_is_refused() {
    let key = key();
    let walk = walk(&key, "projects", "");
    let mut bytes = URL_SAFE_NO_PAD
        .decode(walk.encode(Uuid::now_v7(), now()))
        .unwrap();
    for index in 0..bytes.len() {
        bytes[index] ^= 1;
        let tampered = URL_SAFE_NO_PAD.encode(&bytes);
        assert_eq!(walk.decode(&tampered, now()), None, "byte {index}");
        bytes[index] ^= 1;
    }
}

#[test]
fn a_cursor_is_refused_on_another_list_or_with_other_filters() {
    let key = key();
    let cursor = walk(&key, "projects", "active=true").encode(Uuid::now_v7(), now());
    assert_eq!(
        walk(&key, "clients", "active=true").decode(&cursor, now()),
        None
    );
    assert_eq!(
        walk(&key, "projects", "active=false").decode(&cursor, now()),
        None
    );
}

#[test]
fn a_cursor_older_than_a_day_or_from_the_future_is_refused() {
    let key = key();
    let walk = walk(&key, "projects", "");
    let cursor = walk.encode(Uuid::now_v7(), now());
    assert_eq!(
        walk.decode(&cursor, now() + MAX_AGE + TimeDelta::seconds(1)),
        None
    );
    assert_eq!(walk.decode(&cursor, now() - TimeDelta::minutes(2)), None);
}

#[test]
fn a_cursor_signed_with_another_key_is_refused() {
    let key = key();
    let other = CursorKey::new([8; 32]);
    let cursor = walk(&other, "projects", "").encode(Uuid::now_v7(), now());
    assert_eq!(walk(&key, "projects", "").decode(&cursor, now()), None);
}

#[test]
fn garbage_is_refused() {
    let key = key();
    let walk = walk(&key, "projects", "");
    for cursor in ["", "x", "not base64!", &URL_SAFE_NO_PAD.encode([0; 31])] {
        assert_eq!(walk.decode(cursor, now()), None, "{cursor}");
    }
}

#[test]
fn a_page_is_one_to_a_thousand_rows_and_a_hundred_by_default() {
    let key = key();
    let walk = walk(&key, "projects", "");
    for (limit, expected) in [(None, 100), (Some(1), 1), (Some(1000), 1000)] {
        assert_eq!(
            walk.position(limit, None, now()).unwrap(),
            Position {
                limit: expected,
                after: None
            }
        );
    }
    for limit in [0, 1001] {
        let problem = walk.position(Some(limit), None, now()).unwrap_err();
        assert_eq!((problem.status, problem.code.as_str()), (422, "invalid"));
        assert_eq!(problem.field.as_deref(), Some(&"limit".into()));
    }
}

#[test]
fn a_bad_cursor_answers_invalid_cursor() {
    let key = key();
    let problem = walk(&key, "projects", "")
        .position(None, Some("forged"), now())
        .unwrap_err();
    assert_eq!(
        (problem.status, problem.code.as_str()),
        (400, "invalid_cursor")
    );
}

#[test]
fn the_next_cursor_continues_after_the_last_row_of_the_page() {
    let key = key();
    let walk = walk(&key, "projects", "");
    let ids: Vec<Uuid> = (0..3).map(|_| Uuid::now_v7()).collect();
    let position = Position {
        limit: 2,
        after: None,
    };

    let mut rows = ids.clone();
    let next = walk
        .next_cursor(&mut rows, position, |id| *id, now())
        .unwrap();
    assert_eq!(rows, ids[..2]);
    assert_eq!(
        walk.position(None, Some(&next), now()).unwrap().after,
        Some(ids[1])
    );

    let mut last = ids[..2].to_vec();
    assert_eq!(walk.next_cursor(&mut last, position, |id| *id, now()), None);
    assert_eq!(last, ids[..2], "a last page keeps every row");
}
