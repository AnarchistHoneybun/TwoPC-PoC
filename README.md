# Two Phase Commit

This is a "proof of concept" implementation of a 2-phase commit system.

## Description
2-phase commits describe a protocol for distributed systems to allow for the system's state to be consistent across all nodes after a transaction. 


## System structure 

```

╭─────────────╮    ╭─────────────╮     ╭─────────────╮
│   client    ├────┤ coordinator ├──┬──┤     p_0     ├──╮
╰─────────────╯    ╰──────┬──────╯  │  ╰─────────────╯  │
                          │         │  ╭─────────────╮  │
                          │         ├──┤     p_1     ├──┤
                          │         ┆  ╰─────────────╯  ┆
                          │         ┆  ╭┄┄┄┄┄┄┄┄┄┄┄┄┄╮  ┆
                          │         ╰┄┄┤     p_n     ├┄┄┤
                          │            ╰┄┄┄┄┄┄┄┄┄┄┄┄┄╯  ┆
                          │                             ┆
                          ╰─────────────────────────────╯
```

Each client connects to a coordinator, which is responsible for managing the transaction. The coordinator in turn is connected to participants, which are the nodes that must persist the data being written.

Each write operation then consits of two phases:

### Phase 1: Prepare

When a write request comes in, the coordinator sends a prepare message with the commit details to all participants. The participants check whether or not they are ready to commit a transaction, and respond with a "yes" or "no" vote. Responding with a "yes" also involves the participants persisting the data in a temporary state.

### Phase 2: Commit

The coordinator tallies the votes from the participants. If all participants voted "yes", the coordinator sends a commit message to all participants. The participants then persist the data in a permanent state. If any participant voted "no", the coordinator sends an abort message to all participants, and the participants discard the data. The coordinator then sends a relevant response to the client.


## Utility

2 phase commits allow for a system to be consistent across all nodes after a transaction. This is very useful when allowing clients to read from different arbitrary participants, as the system will always be in a consistent state.