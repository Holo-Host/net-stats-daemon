# Network Statistics

## Context
Collect statistics from holoport and save in mongoDB.

Steps:
- `net-stats-daemon` collects payload on HPOS
- `net-stats-daemon` signs payload and sends to `net-stat-api`
- `net-stats-api` verifies if sender is registered in holoNetwork and uploads payload to mongoDB's `holoport_status` collection

## Technical details:

#### Stats payload
sent in body as stringified json
```
  holoNetwork  # can be one of devNet, alphaNet, flexNet...
  channel      # nix-channel that HPOS is following
  holoportModel # HP or HP+
  sshStatus    # is SSH enabled?
  ztIp         # IP address on Zerotier network
  wanIp        # IPv4 address on internet
  holoportId   # base36 encoded public key of the host
  timestamp    # epoch in secs, updated on API server
```

#### netstatd
PUSH payload service from HPOS
Code of binary [here](https://github.com/Holo-Host/net-stats-daemon).
```robotframework=
Retrieve pub-priv keypair from seed in hpos config file
Collect payload
  holo_network = nixos-option system.holoNetwork | sed -n '2 p' | tr -d \"
  channel = nix-channel --list | grep holo-nixpkgs | cut -d '/' -f 7
  holoport_model = nixos-option system.hpos.target 2>/dev/null | sed -n '2 p' | tr -d \"
  ssh_status = echo $(nixos-option profiles.development.enabl 2>/dev/null | sed -n '2 p' | grep true || echo "false")
  zt_ip = zerotier-cli listnetworks | sed -n '2 p' | awk -F ' ' '{print $NF}' | awk -F ',' '{print $NF}' | awk -F '/' '{print $1}'
  wan_ip = curl https://ipecho.net/plain
  holoport_id_base36 = pub.keypair.into_base36()
Sign payload using ed25519-dalek
POST payload+signature to API
```
Settings of service in nixOs:
```
interval
```

#### net-stat-api
> We've decided to extend https://github.com/Holo-Host/match-service-api-rust/ with new `#post[/stats]` endpoint mounted on `/hosts`

```robotframework=
receive Stats
Verify pubkey in opsconsoledb.registration (requires pub key conversion :-()
Verify signature of payload using pubkey
Update timestamp to server's current UTC
POST to DB host_statistics.holoport_status
```

Stats type:
```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    holo_network: Option<String>,
    channel: Option<String>,
    holoport_model: Option<String>,
    ssh_status: Option<bool>,
    zt_ip: Option<String>,
    wan_ip: Option<String>,
    holoport_model: String,
    timestamp: Option<u64>
}
```

Same payload is incoming in POST and should be posted to mongoDB.

Why `Option<T>`? Because `netstatd` on HPOS might fail to collect data, in that case it will send `null` in failed field. The only field that is required is `holoport_id`.

Service is running on match server:
```
<holo-nixpkgs/profiles/logical/holo/match-server>
```

## DB entries:
#### opsconsoledb.registration
```json
{
  "_id": {
    "$oid": "620386179cf69276b1a8d72a"
  },
  "givenNames": "Peeech",
  "lastName": "Klimek",
  "email": "przemek.klimek@holo.host",
  "isJurisdictionNotInList": false,
  "legalJurisdiction": "Poland",
  "created": {
    "$date": {
      "$numberLong": "1644398103219"
    }
  },
  "oldHoloportIds": [],
  "registrationCode": [
    {
      "code": "e4RNCbr+CsVEs/4AzgIPam4YCM2Py126PJ5ot705m5YuM1FTZJLP+Mv/YkYIOL0TN9SG8Czm5vIxwLa8nGgp7w==",
      "role": "host",
      "agentPubKeys": [
        {
          "pubKey": "uhCAkwMH4_zekbZ3c6jh-iXFenf2fwUU0aGTQQpSPGDGWDzhvTuz9",
          "role": "host"
        }
      ]
    }
  ],
  "__v": {
    "$numberInt": "1"
  }
}
```

#### host_statistics.holoport_status
```json
{
  "holoNetwork": String
  "channel": String
  "holoportModel": String
  "sshStatus": bool
  "ztIp": String
  "wanIp": String
  "holoportIdBase36": String
  "timestamp": i64
}
```


Example of API created in Rust: https://github.com/Holo-Host/match-service-api-rust/
Rocket API Rust: https://rocket.rs/v0.5-rc/guide/

## Discussion


#### what key will we use for signing?
> Answer: we're gonna be using holoport ID for signing payloads from HPOS to API
> Here is why:
  - with what key?
    - 1) Holoport ID?? > If using holochain keys, use Host's hpos (private) key.  Device Bundle exists in the hpos config file: `hp-*.json(hp-primary-pubk5.json)`.
        Note: Can authorize and fetch hpos device bundle by using public `unlock` method exposed in hpos-config : https://github.com/Holo-Host/hpos-config/blob/develop/seed-bundle-explorer/src/lib.rs#L71
          - Pros: Not reliant on external crytography/authentication protocols for internal architecture - makes for easier maintainability
          - Cons: Doesn't protect against misuse of usb key/fraudulant activity
    - 2) If using Zerotier / Network keys, use the node's secret key, located at path: `var/lib/zerotier-one/identity.secret`
          - Pros: Ties authenication to holoport, protecting against fraud, etc.
          - Cons: Tethers internal process to external process - makes more dependent on external project/product changes

  - what creates signature? ~~Lair?~~ > Use ed22519 crate
    - sign` and `verify` signature with ed22519 crate: https://docs.rs/ed25519-dalek/1.0.1/ed25519_dalek/struct.PublicKey.html#method.verify
--
    > Note: The zerotier `identity.secret` key is also encrypted with the Curve25519/Ed25519 elliptic curve variant : https://docs.zerotier.com/zerotier/manual/#213cryptographyaname2_1_3a


>After research and consideration, my personal preference is to actually use the Zerotier node keys, as this seems to be the only way to ensure the call originates from the hpos.
> :person_in_lotus_position: Lisa

> We decided to go with holoport ID because it is proprietary to Holo, is readily available on HPOS and can be cross checked with `opsconsoledb` for Host verification.

#### PUSH or PULL?

- PULL from HPOS initiated by net-stat server (One governs many.)
It implies making multiple (thousands) of calls to HPOS simultaneously, collecting answers locally and making one mongoDB update at the time. Will create spike in server load. Also PULL assumes that net-stat server can ssh into Holoport, which very often can be wrong assumption.

- PUSH from HPOS to net-stat server initiated by HPOS
Each HPOS sends payload to server, which saves this payload in mongoDB. Will not create spikes in server load. Creates options for distributed traffic. Does not require ssh access from net-stat server to HPOS. Will require authentication of node.

> We've decided to go with PUSH solution which does not create bottlenecks and seems to be more reliable because of ssh inconsistency.

#### WAN IP

What is it supposed to tell us? A public static IP of the Holoport on WAN. Can be used e.g. for geolocation.

If we use `ifconfig` way very often it will be 192.168.X.X or similar. We need to either use courtesy service `https://ipecho.net/plain` as long as we cache result, or `dig +short myip.opendns.com @resolver1.opendns.com`

> for some reason dig seems unreliable. Using `curl  https://ipecho.net/plain` for now
