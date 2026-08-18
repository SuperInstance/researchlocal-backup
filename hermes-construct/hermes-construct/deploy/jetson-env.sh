# Jetson memory-constrained profile for hermes-construct
#
# Source this file or let systemd load it via EnvironmentFile.
# Values tuned for 2-4GB Jetson Nano/Orin boards.
#
# Memory tiers:
#   <2GB → rayon=1, tokio=2, rooms=2
#   2-4GB → rayon=2, tokio=2, rooms=3  (these defaults)
#   >4GB → rayon=2, tokio=4, rooms=5

RAYON_NUM_THREADS=2
TOKIO_WORKER_THREADS=2
HERMES_MAX_ROOMS=3
HERMES_CONSERVATION_BUDGET=50.0
