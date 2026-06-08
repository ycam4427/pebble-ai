import type { ChatTurn } from "../../store/appStore";
import QueryResultCard from "./QueryResultCard";
import PlanView from "../actions/PlanView";

export default function MessageBubble({ turn }: { turn: ChatTurn }) {
  const isUser = turn.role === "user";
  return (
    <div className={`turn ${isUser ? "user" : "assistant"}`}>
      <div className="turn-body">
        {!isUser && <div className="who">Pebble</div>}
        {(turn.content || (turn.streaming && (turn.chunks?.length ?? 0) > 0)) && (
          <div
            className={`bubble ${turn.isError ? "error" : ""}`}
            style={
              turn.question
                ? { background: "var(--accent-soft)", borderColor: "var(--accent)" }
                : undefined
            }
          >
            {turn.question && (
              <div
                style={{
                  fontSize: 11,
                  fontWeight: 700,
                  color: "var(--accent-strong)",
                  marginBottom: 4,
                }}
              >
                🪨 Pebble's wondering…
              </div>
            )}
            {turn.streaming && turn.chunks?.length
              ? turn.chunks.map((c, i) => (
                  <span className="tok" key={i}>
                    {c}
                  </span>
                ))
              : turn.content}
          </div>
        )}
        {turn.queryResults?.map((q, i) => (
          <QueryResultCard key={i} result={q} />
        ))}
        {turn.plan && !turn.handled && turn.plan.ops.length > 0 && <PlanView plan={turn.plan} />}
        {turn.plan && turn.handled && (
          <div className="faint" style={{ fontSize: 12, paddingLeft: 4 }}>
            ✓ handled
          </div>
        )}
        {turn.stats && turn.stats.tokens > 0 && (
          <div className="stats">
            <span>{turn.stats.model}</span>
            <span>{turn.stats.tokens} tok</span>
            <span>{turn.stats.tokens_per_sec.toFixed(1)} tok/s</span>
          </div>
        )}
      </div>
    </div>
  );
}
