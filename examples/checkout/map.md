# checkout example

```mermaid
flowchart TD
  log_incoming["log_incoming<br/><i>?</i>"]
  authenticate["authenticate<br/><i>async ?</i>"]
  load_cart["load_cart<br/><i>async ?</i>"]
  require_authenticated["require_authenticated<br/><i>?</i>"]
  check_stock["check_stock<br/><i>?</i>"]
  compute_totals["compute_totals<br/><i>?</i>"]
  apply_discount["apply_discount<br/><i>?</i>"]
  reserve_inventory["reserve_inventory<br/><i>async ?</i>"]
  create_order["create_order<br/><i>async ?</i>"]
  log_outbound["log_outbound<br/><i>?</i>"]

load_cart --> require_authenticated
apply_discount --> reserve_inventory
create_order --> log_outbound
  subgraph prepare ["prepare"]
    direction TB
    log_incoming --> authenticate
    subgraph ingest::ingest ["ingest::ingest"]
      direction TB
      authenticate --> load_cart
    end
  end
  subgraph gate ["gate"]
    direction TB
    check_stock --> compute_totals
    subgraph validation::validate ["validation::validate"]
      direction TB
      require_authenticated --> check_stock
    end
    subgraph pricing::price ["pricing::price"]
      direction TB
      compute_totals --> apply_discount
    end
  end
  subgraph fulfillment::fulfill ["fulfillment::fulfill"]
    direction TB
    reserve_inventory --> create_order
  end

  %% shape legend: rect = Request → Request, rhombus = switching (Request → Response/Branch), stadium = Response → Response
```
