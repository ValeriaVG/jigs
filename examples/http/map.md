# http example

```mermaid
flowchart TD
  log_incoming["log_incoming<br/><i>HttpReq → HttpReq</i>"]
  only_get{"only_get<br/><i>HttpReq → Branch<HttpReq,HttpResp></i>"}
  root{"root<br/><i>HttpReq → HttpResp</i>"}
  hello{"hello<br/><i>HttpReq → HttpResp</i>"}
  not_found{"not_found<br/><i>HttpReq → HttpResp</i>"}
  log_outbound(["log_outbound<br/><i>HttpResp → HttpResp</i>"]])

log_incoming --> only_get
only_get --> route
route --> log_outbound
  subgraph route ["route"]
    direction TB
    root
    hello
    not_found
  end

  %% shape legend: rect = Request → Request, rhombus = switching (Request → Response/Branch), stadium = Response → Response
```
