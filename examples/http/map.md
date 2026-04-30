# http example

```mermaid
flowchart TD
  log_incoming["log_incoming<br/><i>HttpRequest → HttpRequest</i>"]
  only_get{"only_get<br/><i>HttpRequest → Branch<HttpRequest,HttpResponse></i>"}
  root{"root<br/><i>HttpRequest → HttpResponse</i>"}
  hello{"hello<br/><i>HttpRequest → HttpResponse</i>"}
  not_found{"not_found<br/><i>HttpRequest → HttpResponse</i>"}
  log_outbound(["log_outbound<br/><i>HttpResponse → HttpResponse</i>"]])

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
