#!/bin/bash
# Fix all enum usages in connection.rs to match bindgen output

cp connection.rs connection.rs.bak

sed -i 's/qp_init_attr\.qp_type = ibv_qp_type::IBV_QPT_RC;/qp_init_attr.qp_type = ibv_qp_type::IBV_QPT_RC as u32;/' connection.rs
sed -i 's/attr\.qp_state = ibv_qp_state::IBV_QPS_INIT;/attr.qp_state = ibv_qp_state::IBV_QPS_INIT as u32;/' connection.rs
sed -i 's/attr\.qp_state = ibv_qp_state::IBV_QPS_RTR;/attr.qp_state = ibv_qp_state::IBV_QPS_RTR as u32;/' connection.rs
sed -i 's/attr\.qp_state = ibv_qp_state::IBV_QPS_RTS;/attr.qp_state = ibv_qp_state::IBV_QPS_RTS as u32;/' connection.rs

# Fix access flags  
sed -i 's/(IBV_ACCESS_REMOTE_READ/(ibv_access_flags::IBV_ACCESS_REMOTE_READ as u32/' connection.rs
sed -i 's/| IBV_ACCESS_REMOTE_WRITE/| ibv_access_flags::IBV_ACCESS_REMOTE_WRITE as u32/' connection.rs
sed -i 's/| IBV_ACCESS_LOCAL_WRITE/| ibv_access_flags::IBV_ACCESS_LOCAL_WRITE as u32/' connection.rs

# Fix MTU
sed -i 's/attr\.path_mtu = ibv_mtu::IBV_MTU_4096;/attr.path_mtu = ibv_mtu::IBV_MTU_4096 as u32;/' connection.rs

# Fix mask usage
sed -i 's/let mask = ibv_qp_attr_mask::/let mask = /' connection.rs
sed -i 's/| ibv_qp_attr_mask::/| /' connection.rs
sed -i 's/mask\.0 as i32/mask as i32/' connection.rs

# Fix work request opcodes
sed -i 's/wr\.opcode = ibv_wr_opcode::IBV_WR_RDMA_READ;/wr.opcode = ibv_wr_opcode::IBV_WR_RDMA_READ as u32;/' connection.rs
sed -i 's/wr\.opcode = ibv_wr_opcode::IBV_WR_RDMA_WRITE;/wr.opcode = ibv_wr_opcode::IBV_WR_RDMA_WRITE as u32;/' connection.rs

# Fix send flags
sed -i 's/wr\.send_flags = ibv_send_flags::IBV_SEND_SIGNALED\.0;/wr.send_flags = ibv_send_flags::IBV_SEND_SIGNALED as u32;/' connection.rs

# Fix wc status check
sed -i 's/wc\.status != ibv_wc_status::IBV_WC_SUCCESS/wc.status != ibv_wc_status::IBV_WC_SUCCESS as u32/' connection.rs

echo "Fixed connection.rs enum usages"
