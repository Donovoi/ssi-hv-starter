#!/bin/bash
# Fix QP attr mask constants

sed -i 's/IBV_QP_STATE/ibv_qp_attr_mask_IBV_QP_STATE/g' connection.rs
sed -i 's/IBV_QP_PKEY_INDEX/ibv_qp_attr_mask_IBV_QP_PKEY_INDEX/g' connection.rs
sed -i 's/IBV_QP_PORT/ibv_qp_attr_mask_IBV_QP_PORT/g' connection.rs
sed -i 's/IBV_QP_ACCESS_FLAGS/ibv_qp_attr_mask_IBV_QP_ACCESS_FLAGS/g' connection.rs
sed -i 's/IBV_QP_AV/ibv_qp_attr_mask_IBV_QP_AV/g' connection.rs
sed -i 's/IBV_QP_PATH_MTU/ibv_qp_attr_mask_IBV_QP_PATH_MTU/g' connection.rs
sed -i 's/IBV_QP_DEST_QPN/ibv_qp_attr_mask_IBV_QP_DEST_QPN/g' connection.rs
sed -i 's/IBV_QP_RQ_PSN/ibv_qp_attr_mask_IBV_QP_RQ_PSN/g' connection.rs
sed -i 's/IBV_QP_MAX_DEST_RD_ATOMIC/ibv_qp_attr_mask_IBV_QP_MAX_DEST_RD_ATOMIC/g' connection.rs
sed -i 's/IBV_QP_MIN_RNR_TIMER/ibv_qp_attr_mask_IBV_QP_MIN_RNR_TIMER/g' connection.rs
sed -i 's/IBV_QP_TIMEOUT/ibv_qp_attr_mask_IBV_QP_TIMEOUT/g' connection.rs
sed -i 's/IBV_QP_RETRY_CNT/ibv_qp_attr_mask_IBV_QP_RETRY_CNT/g' connection.rs
sed -i 's/IBV_QP_RNR_RETRY/ibv_qp_attr_mask_IBV_QP_RNR_RETRY/g' connection.rs
sed -i 's/IBV_QP_SQ_PSN/ibv_qp_attr_mask_IBV_QP_SQ_PSN/g' connection.rs
sed -i 's/IBV_QP_MAX_QP_RD_ATOMIC/ibv_qp_attr_mask_IBV_QP_MAX_QP_RD_ATOMIC/g' connection.rs

echo "Fixed QP attr mask constants"
