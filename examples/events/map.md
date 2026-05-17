# events bus example

```mermaid
flowchart TD
  log_incoming["log_incoming<br/><i>?</i>"]
  parse["parse<br/><i>?</i>"]
  enrich["enrich<br/><i>?</i>"]
  route["route<br/><i>?</i>"]
  log_outbound["log_outbound<br/><i>?</i>"]

log_incoming --> parse
parse --> enrich
enrich --> route
route --> log_outbound

  %% shape legend: rect = Request → Request, rhombus = switching (Request → Response/Branch), stadium = Response → Response
```
