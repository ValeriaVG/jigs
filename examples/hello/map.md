# hello example

```mermaid
flowchart TD
  log_incoming["log_incoming<br/><i>NameReq → NameReq</i>"]
  validate_incoming{"validate_incoming<br/><i>NameReq → Branch<NameReq,GreetingResp></i>"}
  greet{"greet<br/><i>NameReq → GreetingResp</i>"}
  shout(["shout<br/><i>GreetingResp → GreetingResp</i>"]])
  log_outbound(["log_outbound<br/><i>GreetingResp → GreetingResp</i>"]])

log_incoming --> validate_incoming
validate_incoming --> greet
greet --> shout
shout --> log_outbound

  %% shape legend: rect = Request → Request, rhombus = switching (Request → Response/Branch), stadium = Response → Response
```
