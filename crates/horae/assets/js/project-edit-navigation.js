(() => {
  // Installed before hydration so every same-document history entry is indexed.
  // Keep Dioxus's scroll coordinates in slots 0 and 1 unchanged.
  const position = state => Array.isArray(state) ? state[2]?.horaePosition : state?.horaePosition;
  const tagged = (state, index) => Array.isArray(state)
    ? [state[0], state[1], { horaePosition: index }]
    : { ...state, horaePosition: index };
  const push = history.pushState.bind(history);
  const replace = history.replaceState.bind(history);
  let current = position(history.state) ?? 0;
  // Undoing a rejected back or forward: its popstates stay away from the router.
  let restoring = false;
  // The person chose to leave: the next navigation goes through.
  let leaving = false;
  // Repeats the navigation the open dialog asks about.
  let retry = null;
  replace(tagged(history.state, current), '');

  const editor = () => document.querySelector('[data-editor-state], [data-project-edit-state]');
  const state = () => {
    const found = editor();
    return found?.hasAttribute('data-editor-leaving') || found?.hasAttribute('data-project-edit-leaving')
      ? 'clean' : found?.dataset.editorState ?? found?.dataset.projectEditState;
  };
  // Holds a navigation away from unsaved or unresolved work. The app's
  // `LeaveDialog` says why, and for unsaved changes offers to leave anyway,
  // which calls `horaeLeave`. A native confirm would do, but a browser may
  // suppress it and answer "stay" without showing anything.
  const held = then => {
    if (leaving) {
      leaving = false;
      return false;
    }
    const now = state();
    if (now !== 'dirty' && now !== 'pending') return false;
    retry = now === 'dirty' ? then : null;
    const kind = editor()?.dataset.editorKind ?? 'project';
    dispatchEvent(new CustomEvent('horae-navigation-held', { detail: { state: now, kind } }));
    return true;
  };
  globalThis.horaeLeave = () => {
    const then = retry;
    retry = null;
    if (!then) return;
    leaving = true;
    then();
  };
  // A navigation repeated here bypasses the router, so tell it as the browser would.
  const routed = () => dispatchEvent(new PopStateEvent('popstate', { state: history.state }));

  history.pushState = (data, title, url) => {
    if (held(() => { history.pushState([0, 0], title, url); routed(); })) return;
    push(tagged(data, current + 1), title, url);
    current++;
  };
  history.replaceState = (data, title, url) => {
    // Dioxus also replaces state merely to record the current scroll position.
    if (url != null && new URL(url, location.href).href !== location.href
      && held(() => { history.replaceState([0, 0], title, url); routed(); })) return;
    replace(tagged(data, current), title, url);
  };
  addEventListener('popstate', event => {
    const destination = position(event.state);
    if (restoring) {
      event.stopImmediatePropagation();
      if (destination === current) restoring = false;
      else if (destination != null) history.go(current - destination);
      return;
    }
    if (destination == null) {
      // A native fragment link stays on this screen and creates an untagged entry.
      current++;
      replace(tagged(event.state, current), '');
      return;
    }
    if (destination !== current && held(() => history.go(destination - current))) {
      // Do not let the router unmount the editor while restoring the rejected URL.
      event.stopImmediatePropagation();
      restoring = true;
      history.go(current - destination);
    } else {
      current = destination;
    }
  }, true);
  addEventListener('beforeunload', event => {
    if (state() === 'dirty' || state() === 'pending') {
      event.preventDefault();
      event.returnValue = '';
    }
  });
})();
