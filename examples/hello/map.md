# hello example

```mermaid
flowchart TD
  log_incoming["log_incoming<br/><i>String → String</i>"]
  validate_incoming{"validate_incoming<br/><i>String → Branch<String,String></i>"}
  greet{"greet<br/><i>String → String</i>"}
  shout(["shout<br/><i>String → String</i>"]])
  log_outbound(["log_outbound<br/><i>String → String</i>"]])

log_incoming --> validate_incoming
validate_incoming --> greet
greet --> shout
shout --> log_outbound

  %% shape legend: rect = Request → Request, rhombus = switching (Request → Response/Branch), stadium = Response → Response
```
