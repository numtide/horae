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
  let restoring = false;
  replace(tagged(history.state, current), '');

  const state = () => {
    const editor = document.querySelector('[data-editor-state], [data-project-edit-state]');
    return editor?.hasAttribute('data-editor-leaving') || editor?.hasAttribute('data-project-edit-leaving')
      ? 'clean' : editor?.dataset.editorState ?? editor?.dataset.projectEditState;
  };
  const canLeave = () => {
    const invoice = document.querySelector('[data-editor-kind="invoice"]');
    if (state() === 'pending') {
      alert(invoice ? 'An invoice request is unresolved. Wait for it to finish, or recover the request before leaving.'
        : 'A project save is unresolved. Wait for it to finish, or retry the request before leaving.');
      return false;
    }
    return state() !== 'dirty' || confirm(invoice ? 'Discard unsaved invoice changes?' : 'Discard unsaved project changes?');
  };

  history.pushState = (data, title, url) => {
    if (restoring || !canLeave()) return;
    push(tagged(data, current + 1), title, url);
    current++;
  };
  history.replaceState = (data, title, url) => {
    // Dioxus also replaces state merely to record the current scroll position.
    if (url != null && new URL(url, location.href).href !== location.href
      && (restoring || !canLeave())) return;
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
    if (destination !== current && !canLeave()) {
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
