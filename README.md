src
├── components
│   ├── elements           # Wiederverwendbare UI-Elemente (Buttons, Cards, Inputs)
│   └── layout             # Layout-Komponenten (feature-independent)
│       ├── body.rs        # Grundstruktur / Body Wrapper
│       ├── footer.rs      # Footer Template
│       └── header.rs      # Header Template / Navbar
├── main.rs                # Einstiegspunkt, startet Tokio + Rama HTTP Server
├── pages
│   └── home.rs            # Feature-seite: Home/Landing Page, bindet Templates
├── route
│   ├── http.rs            # Root HTTP Service, delegiert an router.rs
│   ├── mod.rs             # Exportiert Submodule
│   └── router.rs          # Routing via match_service!, verbindet Pages / Flows
├── static
│   ├── assets             # JS / Fonts / Other static files
│   └── styles
│       ├── icons          # Icons
│       └── images         # Bilder
├── utils
│   ├── handler            # Utilities für HTTP Handler
│   │   ├── get.rs         # Standardisierte GET Handler
│   │   ├── post.rs        # Standardisierte POST Handler
│   │   └── sse.rs         # SSE Handler + PatchElements Flow
│   └── wrapper
│       └── sseReciver.rs  # Wrapper für SSE Streams / PatchElements Empfang
