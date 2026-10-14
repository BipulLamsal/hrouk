# hrouk

Know who is on your local network. Fully offline, no internet needed.

Agents announce themselves on the LAN using mDNS. The console asks who they are over an
encrypted connection and checks them against a database that HR
manages. Every login and logout is logged and saved.

## Who does what

* Employee: runs the agent, sends HR their endpoint id (public key). 
* HR: opens the web page, enrolls the employee's email and endpoint id.
* Console: watches the LAN, validates each login, as registered by the HR.

## Timeline

```
1. Employee runs agent     -> prints endpoint id, announces on LAN
2. Employee shares endpoint id -> to HR 
3. HR enrolls              -> email and endpoint id in the web page
4. Agent announces          -> console hears the broadcast
5. Console connects         -> asks WhoAmI over encrypted iroh
6. Agent replies            -> email
7. Console checks MySQL     -> enrolled? AUTHORIZED : UNKNOWN
8. Console logs             -> row in presence_log
9. Agent goes quiet          -> LOGOUT after stale timeout 
```

## Run as employee

```
cargo run -p agent -- --email alice@example.com
```
share your endpoint id to the HR :)

## Run as HR

```
docker compose up -d
cargo run -p console
```

Open `http://127.0.0.1:8080` to enroll users.

### Note
This version currently supports LAN based broadcast, but iroh support end to end connection remotely as well can be expanded. 
