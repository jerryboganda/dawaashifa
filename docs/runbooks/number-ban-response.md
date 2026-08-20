# RUNBOOK — Unofficial WhatsApp Number Ban Response & Failover

## 1. Overview
When an unofficial Baileys number is banned by WhatsApp (Meta), the system automatically detects the `LoggedOut` disconnection event or HTTP 403 response, transitions the channel status to `BANNED`, drains the outbound queue, and fails over to the next active channel in the branch pool.

## 2. Business Identity Isolation (Mandatory)
- **Rule**: Unofficial numbers MUST NEVER join an Official WABA Meta Business Manager account or identity.
- **Enforcement**: Automatic validation in code rejects any configuration associating an `Unofficial` transport with `OfficialWaba`.

## 3. Ban Response Sequence
1. **Detection**:
   - `connection.update` triggers with `DisconnectReason.loggedOut`
   - Health score immediately drops to `0` and status is set to `BANNED`
2. **Failover Execution**:
   - Open conversations in the branch are reassigned to the fallback active number
   - Outbound queue resumes on the fallback channel
   - Alert event `channel.banned` is emitted
3. **Customer Communication Note**:
   - Customers with the banned number saved in their phones cannot send incoming messages to that dead number
   - Operations team should proactively broadcast/SMS the new contact number to active customers

## 4. Unban & Replacement Procedure
1. Provision new SIM number.
2. Initialize sidecar container in `WARMING` state (40 msgs/day cap for 7–14 days).
3. Scan QR code once (auth state persists in Postgres `wa_sessions` table).
4. System automatically promotes channel to `ACTIVE` once health score exceeds 80 after warming period.
