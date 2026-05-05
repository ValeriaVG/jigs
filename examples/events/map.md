# events bus example

```mermaid
flowchart TD
  log_incoming["log_incoming<br/><i>RawEvent → RawEvent</i>"]
  parse{"parse<br/><i>RawEvent → Branch<EventCtx,EventResult></i>"}
  enrich["enrich<br/><i>EventCtx → EventCtx</i>"]
  log_outbound(["log_outbound<br/><i>EventResult → EventResult</i>"]])

log_incoming --> parse
parse --> enrich
enrich --> route
route --> log_outbound
  subgraph route ["route"]
    direction TB
  end

  %% shape legend: rect = Request → Request, rhombus = switching (Request → Response/Branch), stadium = Response → Response
```
