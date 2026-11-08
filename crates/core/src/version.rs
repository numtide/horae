//! The version label a build shows: the release tag for a release build,
//! otherwise `dev` and the commit it was built from.

/// `v0.2.0` for a release, `dev · 1e1971e` (or `1e1971e-dirty`) for any other
/// build with a known commit, and plain `dev` without one.
pub fn label(release_tag: Option<&str>, commit: Option<&str>) -> String {
    match (release_tag, commit) {
        (Some(tag), _) => tag.to_owned(),
        (None, Some(commit)) if !commit.is_empty() => format!("dev · {commit}"),
        _ => "dev".to_owned(),
    }
}

/// Whether a release tag names the crate version it is built from (`v` +
/// `version`). No tag always matches. `const` so the build can refuse a
/// mismatch at compile time.
pub const fn release_tag_matches(release_tag: Option<&str>, version: &str) -> bool {
    let Some(tag) = release_tag else {
        return true;
    };
    let (tag, version) = (tag.as_bytes(), version.as_bytes());
    if tag.len() != version.len() + 1 || tag[0] != b'v' {
        return false;
    }
    // Slice equality is not `const`, hence the loop.
    let mut i = 0;
    while i < version.len() {
        if tag[i + 1] != version[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_build_shows_the_tag() {
        assert_eq!(label(Some("v0.2.0"), Some("1e1971e")), "v0.2.0");
    }

    #[test]
    fn dev_build_shows_the_commit() {
        assert_eq!(label(None, Some("1e1971e")), "dev · 1e1971e");
    }

    #[test]
    fn dirty_dev_build_keeps_the_dirty_marker() {
        assert_eq!(label(None, Some("1e1971e-dirty")), "dev · 1e1971e-dirty");
    }

    #[test]
    fn build_without_git_shows_dev() {
        assert_eq!(label(None, None), "dev");
    }

    #[test]
    fn empty_commit_shows_dev() {
        assert_eq!(label(None, Some("")), "dev");
    }

    #[test]
    fn tag_matching_the_crate_version_is_accepted() {
        assert!(release_tag_matches(Some("v0.2.0"), "0.2.0"));
    }

    #[test]
    fn no_tag_is_accepted() {
        assert!(release_tag_matches(None, "0.2.0"));
    }

    #[test]
    fn tag_for_another_version_is_refused() {
        assert!(!release_tag_matches(Some("v0.2.1"), "0.2.0"));
    }

    #[test]
    fn tag_without_the_v_prefix_is_refused() {
        assert!(!release_tag_matches(Some("0.2.0"), "0.2.0"));
    }

    #[test]
    fn tag_extending_the_version_is_refused() {
        assert!(!release_tag_matches(Some("v0.2.0-rc.1"), "0.2.0"));
    }

    #[test]
    fn empty_tag_is_refused() {
        assert!(!release_tag_matches(Some(""), "0.2.0"));
    }
}
