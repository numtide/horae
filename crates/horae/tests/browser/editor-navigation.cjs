// Unit tests of the shipped navigation script; no server, database or browser.
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { test } = require('node:test');
const { runInNewContext } = require('node:vm');
const source = readFileSync(join(__dirname, '../../assets/js/project-edit-navigation.js'), 'utf8');

function setup(kind, state, legacy = false) {
  const listeners = new Map();
  const held = [];
  const routed = [];
  const moves = [];
  const location = { href: 'https://horae.test/editor' };
  const editor = {
    dataset: legacy ? { projectEditState: state } : { editorKind: kind, editorState: state },
    leaving: false,
    hasAttribute: name => editor.leaving && ['data-editor-leaving', 'data-project-edit-leaving'].includes(name),
  };
  const history = {
    state: [12, 34],
    length: 1,
    pushState(data, _title, url) { this.state = data; this.length++; if (url != null) location.href = new URL(url, location.href).href; },
    replaceState(data, _title, url) { this.state = data; if (url != null) location.href = new URL(url, location.href).href; },
    go(delta) { moves.push(delta); },
  };
  const event = (type, fields) => ({ type, ...fields, prevented: false, stopped: false,
    preventDefault() { this.prevented = true; }, stopImmediatePropagation() { this.stopped = true; } });
  const context = {
    history, location, URL,
    document: { querySelector: selector => selector.includes('data-editor-state') ? editor : null },
    addEventListener: (name, listener) => listeners.set(name, listener),
    // Copies, since objects made inside the script's context fail deepStrictEqual.
    dispatchEvent: dispatched => {
      if (dispatched.type === 'horae-navigation-held') held.push(JSON.parse(JSON.stringify(dispatched.detail)));
      if (dispatched.type === 'popstate') {
        routed.push(JSON.parse(JSON.stringify(dispatched.state)));
        listeners.get('popstate')(dispatched);
      }
    },
    CustomEvent: function (type, init) { return event(type, { detail: init.detail }); },
    PopStateEvent: function (type, init) { return event(type, { state: init.state }); },
  };
  runInNewContext(source, context);
  return { history, location, editor, held, routed, moves,
    leave: () => context.horaeLeave(),
    emit(name, state) {
      const fired = event(name, { state });
      listeners.get(name)(fired);
      return fired;
    },
  };
}

for (const kind of ['permissions', 'client', 'invoice', 'project', 'timesheet']) {
  test(`${kind}: pending navigation preserves history and says why`, () => {
    const ui = setup(kind, 'pending');
    ui.history.pushState([5, 6], '', '/away');
    ui.history.replaceState([5, 6], '', '/away');
    assert.equal(ui.history.length, 1);
    assert.equal(ui.location.href, 'https://horae.test/editor');
    assert.deepEqual(ui.held, [{ state: 'pending', kind }, { state: 'pending', kind }]);
    // Pending work cannot be discarded from the dialog.
    ui.leave();
    assert.equal(ui.history.length, 1);
    const pop = ui.emit('popstate', [0, 0, { horaePosition: -1 }]);
    assert.equal(pop.stopped, true);
    assert.deepEqual(ui.moves, [1]);
    assert.equal(ui.emit('popstate', [0, 0, { horaePosition: 0 }]).stopped, true);
    const unload = ui.emit('beforeunload');
    assert.equal(unload.prevented, true);
    assert.equal(unload.returnValue, '');
    ui.editor.dataset.editorState = 'clean';
    ui.history.pushState([5, 6], '', '/away');
    assert.equal(ui.history.length, 2);
    assert.equal(ui.emit('beforeunload').prevented, false);
  });

  test(`${kind}: dirty navigation waits for an explicit discard, then happens`, () => {
    const ui = setup(kind, 'dirty');
    ui.history.pushState([7, 8], '', '/away');
    assert.equal(ui.history.length, 1);
    assert.deepEqual(ui.held, [{ state: 'dirty', kind }]);
    assert.equal(ui.emit('beforeunload').prevented, true);
    ui.leave();
    assert.equal(ui.history.length, 2);
    assert.equal(ui.location.href, 'https://horae.test/away');
    // The router hears of it as of a back or forward, and opens at the top.
    assert.deepEqual(ui.routed, [[0, 0, { horaePosition: 1 }]]);
    // The discard covered that one navigation only.
    ui.history.pushState([7, 8], '', '/further');
    assert.equal(ui.history.length, 2);
    assert.equal(ui.held.length, 2);
  });
}

test('a rejected back navigation never blocks the next one', () => {
  const ui = setup('invoice', 'dirty');
  const back = ui.emit('popstate', [0, 0, { horaePosition: -1 }]);
  assert.equal(back.stopped, true);
  assert.deepEqual(ui.moves, [1]);
  // A link followed before the restoring popstate arrives still asks.
  ui.history.pushState([0, 0], '', '/clients');
  assert.equal(ui.held.length, 2);
  assert.equal(ui.emit('popstate', [0, 0, { horaePosition: 0 }]).stopped, true);
  ui.leave();
  assert.equal(ui.location.href, 'https://horae.test/clients');
});

test('discarding after a rejected back goes back after all', () => {
  const ui = setup('invoice', 'dirty');
  ui.emit('popstate', [0, 0, { horaePosition: -2 }]);
  ui.emit('popstate', [0, 0, { horaePosition: 0 }]);
  ui.leave();
  assert.deepEqual(ui.moves, [2, -2]);
  // That popstate reaches the router.
  assert.equal(ui.emit('popstate', [0, 0, { horaePosition: -2 }]).stopped, false);
});

for (const kind of ['permissions', 'timesheet']) {
  test(`${kind}: scroll-only replacements preserve Dioxus coordinates without asking`, () => {
    const ui = setup(kind, 'pending');
    ui.history.replaceState([77, 88], '');
    assert.equal(ui.history.state[0], 77);
    assert.equal(ui.history.state[1], 88);
    assert.equal(ui.history.state[2].horaePosition, 0);
    assert.equal(ui.held.length, 0);
  });
}

test('the legacy project opt-in and explicit release keep working', () => {
  const ui = setup('project', 'pending', true);
  ui.history.pushState(null, '', '/away');
  assert.equal(ui.history.length, 1);
  assert.deepEqual(ui.held, [{ state: 'pending', kind: 'project' }]);
  ui.editor.leaving = true;
  ui.history.pushState([0, 0], '', '/away');
  assert.equal(ui.history.length, 2);
  assert.equal(ui.emit('beforeunload').prevented, false);
});
