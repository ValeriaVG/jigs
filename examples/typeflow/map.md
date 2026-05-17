# typeflow example

```mermaid
flowchart TD
  log_incoming["log_incoming<br/><i>RawReq → RawReq</i>"]
  authenticate{"authenticate<br/><i>RawReq → Branch<AuthReq,OutputResp></i>"}
  prepare{"prepare<br/><i>AuthReq → Branch<ComputeReq,OutputResp></i>"}
  calculate{"calculate<br/><i>ComputeReq → OutputResp</i>"}
  log_outbound(["log_outbound<br/><i>OutputResp → OutputResp</i>"]])

log_incoming --> features::auth::authenticate
features::auth::authenticate --> prepare
calculate --> log_outbound
  subgraph features::compute::compute ["features::compute::compute"]
    direction TB
    prepare --> calculate
  end

  %% shape legend: rect = Request → Request, rhombus = switching (Request → Response/Branch), stadium = Response → Response
```
