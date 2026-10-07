const [key, operation, value] = await dioxus.recv();
try {
  const current = sessionStorage.getItem(key);
  const size = text => new TextEncoder().encode(text).length;
  if ((current !== null && size(current) > 524288) ||
      (value !== null && size(value) > 524288)) {
    dioxus.send({Err: 'Permission recovery data exceeds 512 KiB. Keep this tab and check saved permissions.'});
  } else if (operation === 'load') {
    dioxus.send({Ok: current});
  } else if (operation !== 'store' && operation !== 'clear') {
    dioxus.send({Err: 'Unknown permission recovery operation.'});
  } else if (typeof value !== 'string') {
    dioxus.send({Err: 'Missing permission recovery request.'});
  } else if (current !== null && current !== value) {
    dioxus.send({Err: 'Another permission request is unresolved. Reload to recover it before making a new change.'});
  } else {
    // No await between comparison and write: another task cannot replace this slot.
    if (operation === 'store') sessionStorage.setItem(key, value);
    else sessionStorage.removeItem(key);
    dioxus.send({Ok: null});
  }
} catch (_) {
  dioxus.send({Err: 'Browser session storage is unavailable. Keep this tab and retry.'});
}
