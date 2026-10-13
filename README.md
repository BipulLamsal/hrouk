# hrouk

Hrouk is a lan based tooling built with iroh and mDNS, where users can be 
monitored (not now later) by a HR. Right now it only supports details like
its a concept of art: login and logout system for users to let HR know I 
am at the office.

### How it works:

- each employee runs `hrouk-agent` on their laptop
- HR runs `hrouk-console` on their machine
- agent says "hey I'm here" over mDNS 
- console hears it, dials back over encrypted iroh, asks "who are you?"
- agent replies with just an email
- console saves the info

### Why two steps:

- mDNS only shares endpoint id and iorh endpoint address, not the email
- so someone sniffing the LAN sees a device, but not whose it is
- email only goes over the encrypted iroh connection

