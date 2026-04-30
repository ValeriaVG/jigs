# async example

```mermaid
flowchart TD
  log_incoming["log_incoming<br/><i>Ctx → Ctx</i>"]
  enrich["enrich<br/><i>async Ctx → Ctx</i>"]
  require_account{"require_account<br/><i>Ctx → Branch<Ctx,String></i>"}
  render{"render<br/><i>Ctx → String</i>"}
  log_outbound(["log_outbound<br/><i>String → String</i>"]])

log_incoming --> enrich
enrich --> require_account
require_account --> render
render --> log_outbound

  %% shape legend: rect = Request → Request, rhombus = switching (Request → Response/Branch), stadium = Response → Response
```
