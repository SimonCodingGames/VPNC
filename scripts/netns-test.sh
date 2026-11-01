#!/usr/bin/env bash
# Linux-only acceptance harness for classic WireGuard (Schritt 7).
# Requires: root, ip, wg, nft, wireguard module.
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "netns-test.sh: Linux only" >&2
  exit 1
fi
if [[ "${EUID}" -ne 0 ]]; then
  echo "netns-test.sh: run as root" >&2
  exit 1
fi

command -v wg >/dev/null
command -v nft >/dev/null
modprobe wireguard

NS_SRV=vpnc-srv
NS_CLI=vpnc-cli
WG_SRV=wg-s
WG_CLI=wg-c
cleanup() {
  ip netns del "$NS_SRV" 2>/dev/null || true
  ip netns del "$NS_CLI" 2>/dev/null || true
}
trap cleanup EXIT
cleanup

ip netns add "$NS_SRV"
ip netns add "$NS_CLI"

# Keys
SRV_PRIV=$(wg genkey)
SRV_PUB=$(printf '%s' "$SRV_PRIV" | wg pubkey)
CLI_PRIV=$(wg genkey)
CLI_PUB=$(printf '%s' "$CLI_PRIV" | wg pubkey)

ip netns exec "$NS_SRV" ip link add "$WG_SRV" type wireguard
ip netns exec "$NS_CLI" ip link add "$WG_CLI" type wireguard

# Server: 10.66.0.1/24 listen 51820
TMP=$(mktemp)
chmod 600 "$TMP"
cat >"$TMP" <<EOF
[Interface]
PrivateKey = $SRV_PRIV
ListenPort = 51820

[Peer]
PublicKey = $CLI_PUB
AllowedIPs = 10.66.0.2/32
EOF
ip netns exec "$NS_SRV" wg setconf "$WG_SRV" "$TMP"
ip netns exec "$NS_SRV" ip addr add 10.66.0.1/24 dev "$WG_SRV"
ip netns exec "$NS_SRV" ip link set "$WG_SRV" up

# Client endpoint via veth pair
ip link add v-s type veth peer name v-c
ip link set v-s netns "$NS_SRV"
ip link set v-c netns "$NS_CLI"
ip netns exec "$NS_SRV" ip addr add 192.168.200.1/24 dev v-s
ip netns exec "$NS_CLI" ip addr add 192.168.200.2/24 dev v-c
ip netns exec "$NS_SRV" ip link set v-s up
ip netns exec "$NS_CLI" ip link set v-c up
ip netns exec "$NS_SRV" ip link set lo up
ip netns exec "$NS_CLI" ip link set lo up

cat >"$TMP" <<EOF
[Interface]
PrivateKey = $CLI_PRIV
ListenPort = 51821

[Peer]
PublicKey = $SRV_PUB
Endpoint = 192.168.200.1:51820
AllowedIPs = 10.66.0.0/24
PersistentKeepalive = 1
EOF
ip netns exec "$NS_CLI" wg setconf "$WG_CLI" "$TMP"
ip netns exec "$NS_CLI" ip addr add 10.66.0.2/32 dev "$WG_CLI"
ip netns exec "$NS_CLI" ip link set "$WG_CLI" up
rm -f "$TMP"

# Wait for handshake
ok=0
for _ in $(seq 1 20); do
  if ip netns exec "$NS_CLI" ping -c 1 -W 1 10.66.0.1 >/dev/null 2>&1; then
    ok=1
    break
  fi
  sleep 0.5
done

if [[ "$ok" -ne 1 ]]; then
  echo "FAIL: client could not ping server tunnel IP" >&2
  ip netns exec "$NS_CLI" wg show || true
  exit 1
fi

echo "OK: netns classic WireGuard handshake + ping"
