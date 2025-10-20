# Connection.rs fixes - comprehensive enum usage updates

This file documents all the enum type fixes needed in connection.rs to match bindgen output.

## Changes needed:

1. qp_init_attr.qp_type = ibv_qp_type::IBV_QPT_RC
2. attr.qp_state = ibv_qp_state::IBV_QPS_INIT (and RTR, RTS)
3. attr.qp_access_flags = (ibv_access_flags::IBV_ACCESS_REMOTE_READ as u32 | ...) 
4. Use ibv_qp_attr_mask enum values
5. Use ibv_mtu::IBV_MTU_4096
6. Use ibv_wr_opcode::IBV_WR_RDMA_READ/WRITE
7. Use ibv_send_flags::IBV_SEND_SIGNALED
8. Use ibv_wc_status::IBV_WC_SUCCESS

9. Fix _compat_ibv_port_attr field access
