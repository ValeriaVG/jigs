# todo-api example

```mermaid
flowchart TD
  log_incoming["log_incoming<br/><i>?</i>"]
  parse_credentials["parse_credentials<br/><i>?</i>"]
  create_user["create_user<br/><i>?</i>"]
  render_created_token["render_created_token<br/><i>?</i>"]
  verify_credentials["verify_credentials<br/><i>?</i>"]
  render_existing_token["render_existing_token<br/><i>?</i>"]
  not_found["not_found<br/><i>?</i>"]
  authenticate["authenticate<br/><i>?</i>"]
  load_todos["load_todos<br/><i>?</i>"]
  render_many["render_many<br/><i>?</i>"]
  parse_new_todo["parse_new_todo<br/><i>?</i>"]
  insert_todo["insert_todo<br/><i>?</i>"]
  render_one_created["render_one_created<br/><i>?</i>"]
  parse_todo_id["parse_todo_id<br/><i>?</i>"]
  load_todo["load_todo<br/><i>?</i>"]
  render_one["render_one<br/><i>?</i>"]
  parse_todo_update["parse_todo_update<br/><i>?</i>"]
  apply_update["apply_update<br/><i>?</i>"]
  remove_todo["remove_todo<br/><i>?</i>"]
  render_removed["render_removed<br/><i>?</i>"]
  parse_label_op["parse_label_op<br/><i>?</i>"]
  attach["attach<br/><i>?</i>"]
  detach["detach<br/><i>?</i>"]
  load_labels["load_labels<br/><i>?</i>"]
  render_many_labels["render_many_labels<br/><i>?</i>"]
  parse_new_label["parse_new_label<br/><i>?</i>"]
  insert_label["insert_label<br/><i>?</i>"]
  render_one_label_created["render_one_label_created<br/><i>?</i>"]
  parse_label_update["parse_label_update<br/><i>?</i>"]
  apply_label_update["apply_label_update<br/><i>?</i>"]
  render_one_label["render_one_label<br/><i>?</i>"]
  parse_label_id["parse_label_id<br/><i>?</i>"]
  remove_label["remove_label<br/><i>?</i>"]
  render_label_removed["render_label_removed<br/><i>?</i>"]
  log_outbound["log_outbound<br/><i>?</i>"]

log_incoming --> dispatch
dispatch --> log_outbound
  subgraph dispatch ["dispatch"]
    direction TB
    subgraph features::auth::auth ["features::auth::auth"]
      direction TB
      subgraph signup ["signup"]
        direction TB
        parse_credentials --> create_user
        create_user --> render_created_token
      end
      subgraph login ["login"]
        direction TB
        parse_credentials --> verify_credentials
        verify_credentials --> render_existing_token
      end
    end
    subgraph features::todos::todos ["features::todos::todos"]
      direction TB
      subgraph list ["list"]
        direction TB
        auth::authenticate --> load_todos
        load_todos --> render_many
      end
      subgraph create ["create"]
        direction TB
        auth::authenticate --> parse_new_todo
        parse_new_todo --> insert_todo
        insert_todo --> render_one_created
      end
      subgraph get ["get"]
        direction TB
        auth::authenticate --> parse_todo_id
        parse_todo_id --> load_todo
        load_todo --> render_one
      end
      subgraph update ["update"]
        direction TB
        auth::authenticate --> parse_todo_update
        parse_todo_update --> apply_update
        apply_update --> render_one
      end
      subgraph delete ["delete"]
        direction TB
        auth::authenticate --> parse_todo_id
        parse_todo_id --> remove_todo
        remove_todo --> render_removed
      end
      subgraph attach_label ["attach_label"]
        direction TB
        auth::authenticate --> parse_label_op
        parse_label_op --> attach
        attach --> render_one
      end
      subgraph detach_label ["detach_label"]
        direction TB
        auth::authenticate --> parse_label_op
        parse_label_op --> detach
        detach --> render_one
      end
    end
    subgraph features::labels::labels ["features::labels::labels"]
      direction TB
      subgraph list_labels ["list_labels"]
        direction TB
        auth::authenticate --> load_labels
        load_labels --> render_many_labels
      end
      subgraph create_label ["create_label"]
        direction TB
        auth::authenticate --> parse_new_label
        parse_new_label --> insert_label
        insert_label --> render_one_label_created
      end
      subgraph update_label ["update_label"]
        direction TB
        auth::authenticate --> parse_label_update
        parse_label_update --> apply_label_update
        apply_label_update --> render_one_label
      end
      subgraph delete_label ["delete_label"]
        direction TB
        auth::authenticate --> parse_label_id
        parse_label_id --> remove_label
        remove_label --> render_label_removed
      end
    end
    not_found
  end

  %% shape legend: rect = Request → Request, rhombus = switching (Request → Response/Branch), stadium = Response → Response
```
