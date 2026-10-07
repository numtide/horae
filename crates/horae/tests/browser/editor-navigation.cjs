// Unit tests of the shipped navigation script; no server, database or browser.
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { test } = require('node:test');
const { runInNewContext } = require('node:vm');
const source = readFileSync(join(__dirname, '../../assets/js/project-edit-navigation.js'), 'utf8');

function setup(kind, state, legacy = false) {
  const events = new Map();
  const alerts = [];
  const confirmations = [];
  const moves = [];
  const location = { href: 'https://horae.test/editor' };
  const editor = {
    dataset: legacy ? { projectEditState: state } : { editorKind: kind, editorState: state },
    leaving: false,
    hasAttribute: name => editor.leaving && ['data-editor-leaving', 'data-project-edit-leaving'].includes(name),
  };
  let accept = false;
  const history = {
    state: [12, 34],
    length: 1,
    pushState(data, _title, url) { this.state = data; this.length++; if (url != null) location.href = new URL(url, location.href).href; },
    replaceState(data, _title, url) { this.state = data; if (url != null) location.href = new URL(url, location.href).href; },
    go(delta) { moves.push(delta); },
  };
  runInNewContext(source, {
    history, location, URL,
    document: { querySelector: selector => selector.includes('data-editor-state') || selector === `[data-editor-kind="${kind}"]` ? editor : null },
    addEventListener: (name, listener) => events.set(name, listener),
    alert: message => alerts.push(message),
    confirm: message => { confirmations.push(message); return accept; },
  });
  return { history, location, editor, alerts, confirmations, moves,
    accept: () => { accept = true; },
    emit(name, state) {
      const event = { state, prevented: false, stopped: false,
        preventDefault() { this.prevented = true; }, stopImmediatePropagation() { this.stopped = true; } };
      events.get(name)(event);
      return event;
    },
  };
}

for (const [kind, noun] of [['permissions', 'permission'], ['invoice', 'invoice'], ['project', 'project'], ['client', 'client']]) {
  test(`${kind}: pending navigation preserves history until the request is resolved`, () => {
    const ui = setup(kind, 'pending');
    ui.history.pushState([5, 6], '', '/away');
    ui.history.replaceState([5, 6], '', '/away');
    assert.equal(ui.history.length, 1);
    assert.equal(ui.location.href, 'https://horae.test/editor');
    assert.equal(ui.alerts.length, 2);
    assert.ok(ui.alerts.every(message => message.includes(noun)));
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

  test(`${kind}: dirty navigation requires an explicit discard`, () => {
    const ui = setup(kind, 'dirty');
    ui.history.pushState(null, '', '/away');
    assert.equal(ui.history.length, 1);
    assert.equal(ui.confirmations[0], `Discard unsaved ${noun} changes?`);
    assert.equal(ui.emit('beforeunload').prevented, true);
    ui.accept();
    ui.history.pushState([0, 0], '', '/away');
    assert.equal(ui.history.length, 2);
    assert.equal(ui.location.href, 'https://horae.test/away');
  });
}

test('scroll-only replacements preserve Dioxus coordinates without prompting', () => {
  const ui = setup('permissions', 'pending');
  ui.history.replaceState([77, 88], '');
  assert.equal(ui.history.state[0], 77);
  assert.equal(ui.history.state[1], 88);
  assert.equal(ui.history.state[2].horaePosition, 0);
  assert.equal(ui.alerts.length, 0);
});

test('the legacy project opt-in and explicit release keep working', () => {
  const ui = setup('project', 'pending', true);
  ui.history.pushState(null, '', '/away');
  assert.equal(ui.history.length, 1);
  assert.match(ui.alerts[0], /project save/);
  ui.editor.leaving = true;
  ui.history.pushState([0, 0], '', '/away');
  assert.equal(ui.history.length, 2);
  assert.equal(ui.emit('beforeunload').prevented, false);
});
