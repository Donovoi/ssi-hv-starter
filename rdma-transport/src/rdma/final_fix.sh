#!/bin/bash

# Fix enum constants to use type-prefixed names
sed -i 's/ibv_send_flags::IBV_SEND_SIGNALED/ibv_send_flags_IBV_SEND_SIGNALED/g' connection.rs
sed -i 's/ibv_wc_status::IBV_WC_SUCCESS/ibv_wc_status_IBV_WC_SUCCESS/g' connection.rs

# Fix access flags
sed -i 's/ibv_access_flags::IBV_ACCESS_LOCAL_WRITE/ibv_access_flags_IBV_ACCESS_LOCAL_WRITE/g' device.rs
sed -i 's/ibv_access_flags::IBV_ACCESS_REMOTE_READ/ibv_access_flags_IBV_ACCESS_REMOTE_READ/g' device.rs
sed -i 's/ibv_access_flags::IBV_ACCESS_REMOTE_WRITE/ibv_access_flags_IBV_ACCESS_REMOTE_WRITE/g' device.rs
sed -i 's/ibv_access_flags::IBV_ACCESS_LOCAL_WRITE/ibv_access_flags_IBV_ACCESS_LOCAL_WRITE/g' connection.rs
sed -i 's/ibv_access_flags::IBV_ACCESS_REMOTE_READ/ibv_access_flags_IBV_ACCESS_REMOTE_READ/g' connection.rs
sed -i 's/ibv_access_flags::IBV_ACCESS_REMOTE_WRITE/ibv_access_flags_IBV_ACCESS_REMOTE_WRITE/g' connection.rs

# Fix port attr struct type and access
sed -i 's/_compat_ibv_port_attr/ibv_port_attr/g' device.rs

echo "Applied final fixes"
