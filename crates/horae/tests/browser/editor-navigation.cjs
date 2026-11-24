// Unit tests of the shipped navigation script; no server, database or browser.
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { test } = require('node:test');
const { runInNewContext } = require('node:vm');
const source = readFileSync(join(__dirname, '../../assets/js/project-edit-navigation.js'), 'utf8');

function setup(kind, state, legacy = false, { listening = true } = {}) {
  const listeners = new Map();
  const held = [];
  const routed = [];
  const moves = [];
  const scrolls = [];
  const moved = [];
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
    // `LeaveDialog` cancels the event to say it heard it.
    dispatchEvent: dispatched => {
      if (dispatched.type === 'horae-navigation-held' && listening) {
        held.push(JSON.parse(JSON.stringify(dispatched.detail)));
        if (dispatched.cancelable) dispatched.preventDefault();
      }
      if (dispatched.type === 'horae-navigation-moved') moved.push(location.href);
      if (dispatched.type === 'popstate') {
        routed.push(JSON.parse(JSON.stringify(dispatched.state)));
        listeners.get('popstate')(dispatched);
      }
      return !dispatched.prevented;
    },
    CustomEvent: function (type, init = {}) { return event(type, { detail: init.detail, cancelable: !!init.cancelable }); },
    PopStateEvent: function (type, init) { return event(type, { state: init.state }); },
    scrollX: 120, scrollY: 900,
    scrollTo: (x, y) => scrolls.push([x, y]),
    queueMicrotask: run => run(),
  };
  runInNewContext(source, context);
  return { history, location, editor, held, routed, moves, scrolls, moved,
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
  // A link followed before the restoring popstate arrives waits for it, then asks.
  ui.history.pushState([0, 0], '', '/clients');
  assert.equal(ui.held.length, 1);
  assert.equal(ui.history.length, 1);
  assert.equal(ui.emit('popstate', [0, 0, { horaePosition: 0 }]).stopped, true);
  assert.equal(ui.held.length, 2);
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

// A session history as browsers keep it: entries with an index, `go` queued
// until `settle`, and a popstate only when the target entry exists.
function session(state) {
  const entries = [{ state: null, url: '/a' }];
  let index = 0;
  const queue = [];
  const listeners = new Map();
  const heard = [];
  const editor = { dataset: { editorKind: 'invoice', editorState: 'clean' }, hasAttribute: () => false };
  const pop = () => {
    const event = { type: 'popstate', state: entries[index].state, stopped: false,
      stopImmediatePropagation() { this.stopped = true; } };
    listeners.get('popstate')(event);
    if (!event.stopped) heard.push(entries[index].url);
  };
  const history = {
    get state() { return entries[index].state; },
    pushState(data, _title, url) { entries.splice(index + 1, Infinity, { state: data, url }); index++; },
    replaceState(data, _title, url) { entries[index] = { state: data, url: url ?? entries[index].url }; },
    go(delta) { queue.push(delta); },
  };
  const context = {
    history, URL,
    location: { get href() { return `https://horae.test${entries[index].url}`; } },
    document: { querySelector: selector => selector.includes('data-editor-state') ? editor : null },
    addEventListener: (name, listener) => listeners.set(name, listener),
    dispatchEvent: dispatched => {
      if (dispatched.type === 'popstate') {
        listeners.get('popstate')(dispatched);
        heard.push(entries[index].url);
      }
      if (dispatched.cancelable) dispatched.prevented = true;
      return !dispatched.prevented;
    },
    CustomEvent: function (type, init = {}) { return { type, detail: init.detail, cancelable: !!init.cancelable }; },
    PopStateEvent: function (type, init) { return { type, state: init.state, stopImmediatePropagation() {} }; },
    scrollX: 0, scrollY: 0, scrollTo: () => {}, queueMicrotask: run => run(),
  };
  runInNewContext(source, context);
  // The router's own push, as Dioxus does it.
  const navigate = url => history.pushState([0, 0], '', url);
  navigate('/editor');
  editor.dataset.editorState = state;
  // Runs the oldest queued traversal only.
  const step = () => {
    const target = index + queue.shift();
    if (target >= 0 && target < entries.length) { index = target; pop(); }
  };
  return {
    editor, heard, navigate, step,
    url: () => entries[index].url,
    urls: () => entries.map(entry => entry.url),
    go: delta => queue.push(delta),
    settle() { while (queue.length) step(); },
  };
}

test('a navigation while a rejected back is undone waits for the undo, so later Back still works', () => {
  const ui = session('dirty');
  // Back is rejected: the browser is on /a, and the undo is queued, not yet run.
  ui.go(-1);
  ui.step();
  assert.equal(ui.url(), '/a');
  // Then the save finishes and the app moves on, before the undo runs.
  ui.editor.dataset.editorState = 'clean';
  ui.navigate('/done');
  ui.settle();
  assert.equal(ui.url(), '/done');
  assert.deepEqual(ui.urls(), ['/a', '/editor', '/done']);
  assert.equal(ui.heard.at(-1), '/done');
  // Back from a later page reaches the router.
  ui.navigate('/next');
  ui.go(-1);
  ui.settle();
  assert.equal(ui.url(), '/done');
  assert.deepEqual(ui.heard.slice(-1), ['/done']);
  assert.equal(ui.heard.length, 2);
});

test('with nobody to explain it, a navigation is let through rather than held silently', () => {
  for (const state of ['dirty', 'pending']) {
    const ui = setup('invoice', state, false, { listening: false });
    ui.history.pushState([5, 6], '', '/away');
    assert.equal(ui.location.href, 'https://horae.test/away');
    assert.equal(ui.emit('popstate', [0, 0, { horaePosition: -1 }]).stopped, false);
  }
});

test('a navigation that goes ahead after all closes the dialog that asked', () => {
  const ui = setup('invoice', 'pending');
  ui.history.pushState([0, 0], '', '/away');
  assert.deepEqual(ui.moved, []);
  // The save ends and the app moves on by itself.
  ui.editor.dataset.editorState = 'clean';
  ui.history.pushState([0, 0], '', '/saved');
  ui.history.replaceState([0, 0], '', '/saved?tab=1');
  ui.history.replaceState([3, 4], '');
  assert.deepEqual(ui.moved, ['https://horae.test/saved', 'https://horae.test/saved?tab=1']);
  ui.emit('popstate', [0, 0, { horaePosition: 0 }]);
  assert.equal(ui.moved.length, 3);
});

test('a held link keeps the editor where it was scrolled', () => {
  const ui = setup('invoice', 'dirty');
  ui.history.pushState([0, 0], '', '/away');
  assert.deepEqual(ui.scrolls, [[120, 900]]);
  ui.leave();
  assert.deepEqual(ui.scrolls, [[120, 900]]);
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
