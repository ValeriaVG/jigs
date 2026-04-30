# events bus example

```mermaid
flowchart TD
  log_incoming["log_incoming<br/><i>RawEvent → RawEvent</i>"]
  parse{"parse<br/><i>RawEvent → Branch<EventCtx,EventResult></i>"}
  enrich["enrich<br/><i>EventCtx → EventCtx</i>"]
  validate_order{"validate_order<br/><i>EventCtx → Branch<EventCtx,EventResult></i>"}
  build_result{"build_result<br/><i>EventCtx → EventResult</i>"}
  validate_inventory{"validate_inventory<br/><i>EventCtx → Branch<EventCtx,EventResult></i>"}
  validate_notification{"validate_notification<br/><i>EventCtx → Branch<EventCtx,EventResult></i>"}
  log_outbound(["log_outbound<br/><i>EventResult → EventResult</i>"]])

log_incoming --> parse
parse --> enrich
enrich --> route
route --> log_outbound
  subgraph route ["route"]
    direction TB
    subgraph orders::handle ["orders::handle"]
      direction TB
      validate_order --> build_result
    end
    subgraph inventory::handle ["inventory::handle"]
      direction TB
      validate_inventory --> build_result
    end
    subgraph notifications::handle ["notifications::handle"]
      direction TB
      validate_notification --> build_result
    end
  end

  %% shape legend: rect = Request → Request, rhombus = switching (Request → Response/Branch), stadium = Response → Response
```
