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
  // History changes asked for while undoing, run once the undo lands. Run
  // earlier, a push would drop the entry the undo is heading for, and the
  // undo would never land.
  const deferred = [];
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
    const kind = editor()?.dataset.editorKind ?? 'project';
    // The dialog cancels the event to say it heard it. If nothing did, holding
    // the navigation would block it without a word, so let it through.
    const heard = !dispatchEvent(new CustomEvent('horae-navigation-held', { detail: { state: now, kind }, cancelable: true }));
    retry = heard && now === 'dirty' ? then : null;
    return heard;
  };
  // Dioxus scrolls to the top after a push or replace it thinks happened.
  const keepScroll = () => {
    const [x, y] = [scrollX, scrollY];
    queueMicrotask(() => scrollTo(x, y));
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
  // A navigation went ahead after all (the app moved on once a save ended):
  // a dialog still asking about the page it left closes.
  const moved = () => dispatchEvent(new CustomEvent('horae-navigation-moved'));

  history.pushState = (data, title, url) => {
    const again = () => {
      const at = current;
      history.pushState([0, 0], title, url);
      if (current !== at) routed();
    };
    if (restoring) {
      deferred.push(again);
      keepScroll();
    } else if (held(again)) {
      keepScroll();
    } else {
      push(tagged(data, current + 1), title, url);
      current++;
      moved();
    }
  };
  history.replaceState = (data, title, url) => {
    // Dioxus also replaces state merely to record the current scroll position.
    const moves = url != null && new URL(url, location.href).href !== location.href;
    const again = () => {
      const at = location.href;
      history.replaceState(moves ? [0, 0] : data, title, url);
      if (location.href !== at) routed();
    };
    if (restoring) {
      deferred.push(again);
      if (moves) keepScroll();
    } else if (moves && held(again)) {
      keepScroll();
    } else {
      replace(tagged(data, current), title, url);
      if (moves) moved();
    }
  };
  addEventListener('popstate', event => {
    const destination = position(event.state);
    if (restoring) {
      event.stopImmediatePropagation();
      if (destination === current) {
        restoring = false;
        for (const run of deferred.splice(0)) run();
      } else if (destination != null) history.go(current - destination);
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
      if (destination !== current) moved();
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
