export type DaoId = string;

export interface DaoProfile {
  name: string;
  sector: string;
  registered_at: string;
}

export interface Dao {
  id: DaoId;
  public_key: number[];
  profile: DaoProfile;
  token_balance: number;
}

export type TransactionType = "Invoice" | "Payment";
export type TransactionStatus = "Unmatched" | "Matched" | "Settled" | "Invalid";

export interface Transaction {
  id: string;
  tx_type: TransactionType;
  from: DaoId;
  to: DaoId;
  amount: number;
  currency: string;
  description: string;
  timestamp: string;
  status: TransactionStatus;
  signature: number[];
  invoice_ref: string | null;
  sequence_number: number;
}

export interface Health {
  status: "ok";
  height: number;
  pending: number;
  tick: number;
  registered_daos: number;
  heartbeat_ms: number;
  total_token_supply: number;
}

export interface HeartbeatResponse {
  ticks_run: number;
  solstices_fired: number;
  final_tick: number;
  pending: number;
}

export interface Balance {
  dao_id: DaoId;
  token_balance: number;
  side_chain_height: number;
  closing_balance: {
    accounts_receivable: number;
    accounts_payable: number;
    revenue: number;
    expenses: number;
  };
}

export interface Reputation {
  dao_id: DaoId;
  pagerank: number;
  baseline: number;
  trust: number;
}

export interface NetworkMetrics {
  total_daos: number;
  anxiety: number;
  heartbeat_ms: number;
  adrenaline: number;
  total_token_supply: number;
}

export interface JsonicClientOptions {
  baseUrl?: string;
  fetchImpl?: typeof fetch;
}

export class JsonicError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "JsonicError";
    this.status = status;
  }
}

export class JsonicClient {
  readonly baseUrl: string;
  private readonly fetchImpl: typeof fetch;

  constructor(options: JsonicClientOptions = {}) {
    this.baseUrl = (options.baseUrl ?? "http://127.0.0.1:8080").replace(/\/$/, "");
    this.fetchImpl = options.fetchImpl ?? fetch;
  }

  health(): Promise<Health> {
    return this.request("GET", "/health");
  }

  listDaos(): Promise<Dao[]> {
    return this.request("GET", "/daos");
  }

  registerDao(dao: Dao): Promise<{ dao_id: DaoId }> {
    return this.request("POST", "/daos", dao);
  }

  submitTransaction(transaction: Transaction): Promise<void> {
    return this.request("POST", "/transactions", transaction);
  }

  runHeartbeats(ticks: number): Promise<HeartbeatResponse> {
    return this.request("POST", "/heartbeats", { ticks });
  }

  getBlock(height: number): Promise<unknown> {
    return this.request("GET", `/blocks/${height}`);
  }

  getMetrics(): Promise<NetworkMetrics> {
    return this.request("GET", "/metrics");
  }

  getBalance(daoId: DaoId): Promise<Balance> {
    return this.request("GET", `/balance/${encodeURIComponent(daoId)}`);
  }

  getReputation(daoId: DaoId): Promise<Reputation> {
    return this.request("GET", `/reputation/${encodeURIComponent(daoId)}`);
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const response = await this.fetchImpl(`${this.baseUrl}${path}`, {
      method,
      headers: body === undefined ? undefined : { "content-type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body)
    });

    if (!response.ok) {
      let message = response.statusText;
      try {
        const errorBody = (await response.json()) as { error?: string };
        message = errorBody.error ?? message;
      } catch {
        // Keep the HTTP status text when the node did not return JSON.
      }
      throw new JsonicError(response.status, message);
    }

    if (response.status === 202 || response.headers.get("content-length") === "0") {
      return undefined as T;
    }

    return (await response.json()) as T;
  }
}
