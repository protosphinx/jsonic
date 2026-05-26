import test from "node:test";
import assert from "node:assert/strict";
import { JsonicClient, JsonicError } from "./index.js";

function jsonResponse(body: unknown, init: ResponseInit = {}) {
  return new Response(JSON.stringify(body), {
    status: init.status ?? 200,
    headers: {
      "content-type": "application/json",
      ...init.headers
    }
  });
}

test("JsonicClient trims trailing slash and reads health", async () => {
  const calls: Array<{ url: string; init: RequestInit }> = [];
  const fetchImpl = async (url: string | URL | Request, init?: RequestInit) => {
    calls.push({ url: String(url), init: init ?? {} });
    return jsonResponse({ status: "ok", height: 7, pending: 2, tick: 11 });
  };

  const client = new JsonicClient({
    baseUrl: "http://node.example/",
    fetchImpl: fetchImpl as typeof fetch
  });
  const health = await client.health();

  assert.deepEqual(health, { status: "ok", height: 7, pending: 2, tick: 11 });
  assert.equal(calls[0]?.url, "http://node.example/health");
  assert.equal(calls[0]?.init.method, "GET");
});

test("JsonicClient serializes heartbeat requests", async () => {
  let requestBody: string | undefined;
  const fetchImpl = async (_url: string | URL | Request, init?: RequestInit) => {
    requestBody = init?.body as string | undefined;
    return jsonResponse({
      ticks_run: 3,
      solstices_fired: 1,
      final_tick: 3,
      pending: 0
    });
  };

  const client = new JsonicClient({ fetchImpl: fetchImpl as typeof fetch });
  const result = await client.runHeartbeats(3);

  assert.equal(requestBody, JSON.stringify({ ticks: 3 }));
  assert.equal(result.solstices_fired, 1);
});

test("JsonicClient treats accepted transaction submission as void", async () => {
  const fetchImpl = async () => new Response(null, { status: 202 });
  const client = new JsonicClient({ fetchImpl: fetchImpl as typeof fetch });

  const result = await client.submitTransaction({
    id: "tx1",
    tx_type: "Invoice",
    from: "dao1",
    to: "dao2",
    amount: 100,
    currency: "USD",
    description: "test",
    timestamp: new Date(0).toISOString(),
    status: "Unmatched",
    signature: [],
    invoice_ref: null,
    sequence_number: 1
  });

  assert.equal(result, undefined);
});

test("JsonicClient raises JsonicError with node message", async () => {
  const fetchImpl = async () => jsonResponse({ error: "bad sequence" }, { status: 400 });
  const client = new JsonicClient({ fetchImpl: fetchImpl as typeof fetch });

  await assert.rejects(() => client.health(), (err: unknown) => {
    assert.ok(err instanceof JsonicError);
    assert.equal(err.status, 400);
    assert.equal(err.message, "bad sequence");
    return true;
  });
});
