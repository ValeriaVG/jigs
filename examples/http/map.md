# http example

```mermaid
flowchart TD
  log_incoming["log_incoming<br/><i>?</i>"]
  only_get["only_get<br/><i>?</i>"]
  root["root<br/><i>?</i>"]
  hello["hello<br/><i>?</i>"]
  not_found["not_found<br/><i>?</i>"]
  log_outbound["log_outbound<br/><i>?</i>"]

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
