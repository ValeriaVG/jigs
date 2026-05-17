# async example

```mermaid
flowchart TD
  log_incoming["log_incoming<br/><i>AppReq → AppReq</i>"]
  enrich["enrich<br/><i>async AppReq → AppReq</i>"]
  require_account{"require_account<br/><i>AppReq → Branch<AppReq,AppResp></i>"}
  render{"render<br/><i>AppReq → AppResp</i>"}
  log_outbound(["log_outbound<br/><i>AppResp → AppResp</i>"]])

log_incoming --> enrich
enrich --> require_account
require_account --> render
render --> log_outbound

  %% shape legend: rect = Request → Request, rhombus = switching (Request → Response/Branch), stadium = Response → Response
```
