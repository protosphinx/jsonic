#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { JsonicClient } from "@protosphinx/jsonic-sdk";

const client = new JsonicClient({
  baseUrl: process.env.JSONIC_RPC_URL ?? "http://127.0.0.1:8080"
});

const server = new McpServer({
  name: "jsonic",
  version: "0.1.0"
});

function asText(value: unknown) {
  return {
    content: [
      {
        type: "text" as const,
        text: JSON.stringify(value, null, 2)
      }
    ]
  };
}

server.tool("jsonic_health", "Read Jsonic node health", {}, async () => {
  return asText(await client.health());
});

server.tool("jsonic_list_daos", "List registered Jsonic DAOs", {}, async () => {
  return asText(await client.listDaos());
});

server.tool(
  "jsonic_run_heartbeats",
  "Advance the Jsonic node heartbeat state machine",
  {
    ticks: z.number().int().positive()
  },
  async ({ ticks }) => asText(await client.runHeartbeats(ticks))
);

server.tool(
  "jsonic_get_block",
  "Fetch a Jsonic main-chain block by height",
  {
    height: z.number().int().nonnegative()
  },
  async ({ height }) => asText(await client.getBlock(height))
);

server.tool("jsonic_get_metrics", "Read latest Jsonic network metrics", {}, async () => {
  return asText(await client.getMetrics());
});

server.tool(
  "jsonic_get_balance",
  "Read a DAO token balance and side-chain balance sheet",
  {
    dao_id: z.string().min(1)
  },
  async ({ dao_id }) => asText(await client.getBalance(dao_id))
);

server.tool(
  "jsonic_get_reputation",
  "Read a DAO PageRank, baseline, and trust score",
  {
    dao_id: z.string().min(1)
  },
  async ({ dao_id }) => asText(await client.getReputation(dao_id))
);

const transport = new StdioServerTransport();
await server.connect(transport);
