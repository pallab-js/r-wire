# AuraCap Roadmap: Outshining Wireshark

This document outlines AuraCap's strategic roadmap to become the preferred network analysis tool for both beginners and professionals.

---

## Vision Statement

> _"Network analysis shouldn't require a PhD. AuraCap brings professional-grade packet analysis to everyone through clarity, speed, and beautiful design."_

---

## Competitive Analysis

### What Wireshark Does Well

- Comprehensive protocol support (1000+ dissectors)
- TCP stream reassembly
- Expert info system
- PCAP import/export
- Massive community and documentation

### What Wireshark Does Poorly

- Steep learning curve
- Dated, overwhelming UI
- Slow with large captures
- No native mobile support
- Complex configuration

---

## Roadmap: v0.2.0 - "First Impression" (Q2 2026)

### Goals: Reduce barriers to entry

| Feature            | Description                   | Impact                   | Status  |
| ------------------ | ----------------------------- | ------------------------ | ------- |
| Guided Tour        | First-launch walkthrough      | Beginner onboarding      | ✅ Done |
| Smart Filters      | Natural language filter input | No BPF knowledge needed  | ✅ Done |
| One-Click Analysis | Auto-analyze and summarize    | Zero-friction experience | ✅ Done |

### Technical

- [x] Interactive onboarding wizard (`OnboardingTour.svelte`)
- [x] Filter autocomplete with suggestions
- [x] Quick actions toolbar (Analyze button)

---

## Roadmap: v0.3.0 - "Professional Polish" (Q3 2026)

### Goals: Match Wireshark's core capabilities

| Feature             | Description                  | Impact                |
| ------------------- | ---------------------------- | --------------------- |
| TLS Decryption      | Decrypt HTTPS with key       | See encrypted traffic |
| Protocol Statistics | Protocol distribution charts | Network overview      |
| Export Tools        | Export to CSV, JSON, PDF     | Reporting made easy   |
| Bookmarks           | Save interesting packets     | Easy reference        |

### Technical

- [ ] TLS 1.3 decryption (with private key)
- [ ] Protocol hierarchy statistics
- [ ] Multi-format export (CSV, JSON, PDF)
- [ ] Packet bookmarking system

---

## Roadmap: v0.4.0 - "Power User" (Q4 2026)

### Goals: Outperform Wireshark for power users

| Feature                | Description             | Impact               |
| ---------------------- | ----------------------- | -------------------- |
| Lua Scripting          | Extend with Lua plugins | Custom analysis      |
| Python API             | Programmatic control    | Automation           |
| Remote Capture         | Analyze remote PCAPs    | Distributed analysis |
| Collaborative Analysis | Share capture sessions  | Team workflows       |

### Technical

- [ ] Lua plugin engine
- [ ] Python SDK (pip install auracap)
- [ ] SSH-based remote capture
- [ ] Session file format (.acs)

---

## Roadmap: v0.5.0 - "Enterprise Ready" (2027)

### Goals: Scale for production environments

| Feature           | Description               | Impact                |
| ----------------- | ------------------------- | --------------------- |
| ~~Dark Mode~~     | ✅ Done (v0.2.0)          | Eye comfort           |
| Multi-Window      | Detached packet views     | Multi-monitor         |
| Capture Scheduler | Automated captures        | Continuous monitoring |
| LDAP/SSO Auth     | Enterprise authentication | Team management       |

### Technical

- [x] Theme switcher (light/dark)
- [ ] Multi-window support
- [ ] Scheduled capture jobs
- [ ] Enterprise auth integration

---

## Differentiation Strategy

### 1. **Readability First**

```
Wireshark:   "TCP 443 > 52234 [ACK] Seq=1 Ack=2 Win=65535 Len=0"
AuraCap:     "Your Mac received an encrypted acknowledgment from github.com"
```

### 2. **Performance**

- Lazy loading for millions of packets
- Virtual scrolling in packet list
- Background packet processing
- Memory-efficient Rust core

### 3. **Modern UX**

- Responsive design (mobile-friendly)
- Keyboard-driven workflows
- Touch support for tablets
- Accessibility (WCAG 2.1 AA)

### 4. **Community**

- Discord server for support
- YouTube tutorial series
- Monthly release streams
- Public roadmap voting

---

## Milestones

| Version | Target  | Theme               | Key Deliverable                                       |
| ------- | ------- | ------------------- | ----------------------------------------------------- |
| v0.1.0  | ✅ Done | Foundation          | Core packet capture                                   |
| v0.2.0  | ✅ Done | First Impression    | Beginner-friendly UX + Dark Mode + Enhanced Protocols |
| v0.3.0  | Q3 2026 | Professional Polish | TLS decryption + bookmarks                            |
| v0.4.0  | Q4 2026 | Power User          | Automation & scripting                                |
| v0.5.0  | 2027    | Enterprise          | Multi-window + collaboration                          |
| v1.0.0  | 2027    | Release             | Public 1.0 launch                                     |

---

## How to Contribute to Roadmap

1. **Upvote Issues** - Vote on features at https://github.com/pallab-js/r-wire/issues
2. **Propose Ideas** - Open a discussion at https://github.com/pallab-js/r-wire/discussions
3. **Implement** - Pick a "help wanted" issue and submit a PR
4. **Beta Test** - Join our beta program for early access

---

## Success Metrics

| Metric                     | Current | v0.5.0 Target |
| -------------------------- | ------- | ------------- |
| GitHub Stars               | TBD     | 1,000         |
| Downloads/month            | TBD     | 5,000         |
| Avg. time to first capture | ~10 min | ~2 min        |
| User satisfaction          | TBD     | >90%          |

---

<div align="center">

**Join us in making network analysis accessible to everyone.**

</div>
