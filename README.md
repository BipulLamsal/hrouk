# hrouk

hrouk shows who is on your local network. It runs on your local network with no internet needed.

An agent runs on each employee laptop and announces itself on the LAN. The console hears that announcement, asks the agent who it is over an encrypted iroh connection, and checks the answer against a list managed by HR. Every login and logout is written to a simple MySQL database.

## Roles

An employee runs the agent with their email and sends the printed endpoint id to HR. HR opens the console web page and enrolls that email together with the endpoint id. The console watches the LAN, validates each login as authorized or unknown, and records presence.

## Flow

First the employee starts the agent, which prints its endpoint id and starts announcing on mDNS. The employee shares that id with HR out of band. HR saves it with the employee email in the users table.

When the console hears the announcement, it connects to the agent and requests identity. The agent replies with its email. The console checks whether that email and endpoint id pair exists in the database. It then inserts a login row into presence_log. If the agent goes quiet past the stale timeout, or its mDNS entry expires, the console inserts a logout row.

## Running it

Run as employee:

    cargo run -p agent -- --email alice@example.com

Copy the endpoint id it prints and give it to HR.

Run as HR:

    docker compose up -d
    cargo run -p console

Open `http://127.0.0.1:8080` to enroll users and view status.

## What is implemented

Employee management lives in console/src/registry.rs. It owns the users table and exposes register, users, validate, and status endpoints. The HR web UI calls these endpoints.

Attendance lives in console/src/main.rs. It owns presence detection through mDNS plus WhoAmI, and writes login and logout rows to presence_log. Online means the last event for that email is a login.

The agent owns its own identity and nothing else. It announces on the LAN and answers identity requests. The proto crate only holds the shared message format and key handling, so agent and console stay decoupled and MySQL remains the source of truth.

## Data

The schema is in schema.sql. The users table has email, endpoint id, and display name, one row per enrolled employee. The presence_log table has email, endpoint id, event type, authorized flag, and timestamp, one row per check in or check out.

## API

Enroll a user, list users, check presence, and validate a pair:

    curl "http://127.0.0.1:8080/register?email=alice@example.com&endpoint_id=IDENT&display_name=Alice"
    curl http://127.0.0.1:8080/users
    curl http://127.0.0.1:8080/status
    curl "http://127.0.0.1:8080/validate?email=alice@example.com&endpoint_id=IDENT"

You can also do the same flow from the web page form and watch the status list update. Stop the agent and a logout appears after the timeout.

## Limits and next steps

Right now this only works on one local network. The agent email is taken on trust and matched to the saved list. The endpoint id is shared by hand.

This can grow into Upwork style monitoring. The agent can also report mouse activity, keys pressed, tabs open, and screenshots on a timer. That gives real time tracking on top of login and logout.
