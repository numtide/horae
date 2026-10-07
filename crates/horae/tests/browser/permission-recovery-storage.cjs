// Unit tests of the shipped Dioxus script, not browser acceptance.
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const script = fs.readFileSync(path.join(__dirname, '../../assets/js/permission-recovery-storage.js'), 'utf8');
const key = 'horae-permission-request:v1:org:user';

async function run(storage, operation, value = null, failure = null) {
  let reply;
  await vm.runInNewContext(`(async () => { ${script} })()`, {
    dioxus: { recv: async () => [key, operation, value], send: result => { reply = result; } },
    sessionStorage: {
      getItem(k) { if (failure === 'read') throw Error(); return storage.get(k) ?? null; },
      setItem(k, v) { if (failure === 'write') throw Error(); storage.set(k, v); },
      removeItem(k) { if (failure === 'clear') throw Error(); storage.delete(k); },
    },
    TextEncoder,
  });
  return JSON.parse(JSON.stringify(reply));
}

test('store acknowledgement precedes a load from a new execution', async () => {
  const storage = new Map();
  assert.deepEqual(await run(storage, 'store', 'original'), { Ok: null });
  assert.deepEqual(await run(storage, 'load'), { Ok: 'original' });
});
test('identical store is idempotent; another request cannot replace it', async () => {
  const storage = new Map([[key, 'original']]);
  assert.deepEqual(await run(storage, 'store', 'original'), { Ok: null });
  assert.ok((await run(storage, 'store', 'different')).Err);
  assert.equal(storage.get(key), 'original');
});
test('clear requires the exact stored record and is idempotent', async () => {
  const storage = new Map([[key, 'original']]);
  assert.ok((await run(storage, 'clear', 'different')).Err);
  assert.equal(storage.get(key), 'original');
  assert.deepEqual(await run(storage, 'clear', 'original'), { Ok: null });
  assert.deepEqual(await run(storage, 'clear', 'original'), { Ok: null });
});
test('storage errors never acknowledge a mutation or erase a request', async () => {
  const storage = new Map([[key, 'original']]);
  for (const [operation, failure] of [['load', 'read'], ['store', 'write'], ['clear', 'clear']]) {
    assert.ok((await run(storage, operation, 'original', failure)).Err);
    assert.equal(storage.get(key), 'original');
  }
});
test('size limit counts UTF-8 bytes on both reads and writes', async () => {
  const storage = new Map();
  const oversized = 'é'.repeat(262145);
  assert.ok((await run(storage, 'store', oversized)).Err);
  assert.equal(storage.size, 0);
  storage.set(key, oversized);
  assert.ok((await run(storage, 'load')).Err);
  assert.equal(storage.get(key), oversized);
});
test('other requesters and unrelated browser records are untouched', async () => {
  const storage = new Map([['horae-permission-request:v1:org:other', 'other'], ['unrelated', 'value']]);
  assert.deepEqual(await run(storage, 'load'), { Ok: null });
  await run(storage, 'store', 'original');
  await run(storage, 'clear', 'original');
  assert.deepEqual([...storage], [['horae-permission-request:v1:org:other', 'other'], ['unrelated', 'value']]);
});
