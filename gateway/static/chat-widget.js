/* ═══════════════════════════════════════
   ⚕️ Eir Chat Widget — Embedded in OpenEMR
   Injected by Eir Gateway into proxied pages
   Powered by Asgard AI
   ═══════════════════════════════════════ */

(function() {
    'use strict';

    // ── Guard: only run once, only in top window, only after login ──

    // Don't load in iframes/frames — OpenEMR uses frames after login
    try { if (window.self !== window.top) return; } catch(e) { return; }

    // Prevent double-init across frames (use top window's flag)
    try {
        if (window.top.__eirChatLoaded) return;
        window.top.__eirChatLoaded = true;
    } catch(e) {
        if (window.__eirChatLoaded) return;
        window.__eirChatLoaded = true;
    }

    // Prevent duplicate DOM elements (extra safety)
    if (document.getElementById('eir-chat-fab')) return;

    // Don't show on login page — only show after authentication
    if (document.querySelector('input[name="authUser"]') ||
        document.querySelector('.login-box') ||
        location.pathname.includes('/login/') ||
        location.pathname.includes('login.php')) return;

    // Auto-detect: if we're on the gateway (:8300), use relative URLs.
    // If on OpenEMR directly (:80 etc.), use absolute gateway URL.
    const GATEWAY = window.EIR_GATEWAY_URL || (location.port === '8300' ? '' : 'http://localhost:8300');
    const CHAT_URL = GATEWAY + '/chat';
    const STATUS_URL = GATEWAY + '/v1/chat/status';

    // ── Styles ──
    const style = document.createElement('style');
    style.textContent = `
        #eir-chat-fab {
            position: fixed;
            bottom: 24px;
            right: 24px;
            width: 56px;
            height: 56px;
            border-radius: 50%;
            background: linear-gradient(135deg, #22c55e, #16a34a);
            border: none;
            cursor: pointer;
            box-shadow: 0 4px 20px rgba(34, 197, 94, 0.4);
            z-index: 99999;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 28px;
            transition: all 0.3s ease;
            animation: eir-pulse 2s infinite;
        }
        #eir-chat-fab:hover {
            transform: scale(1.1);
            box-shadow: 0 6px 30px rgba(34, 197, 94, 0.6);
        }
        #eir-chat-fab.open {
            animation: none;
            background: linear-gradient(135deg, #6b7280, #4b5563);
            box-shadow: 0 4px 20px rgba(107, 114, 128, 0.4);
        }
        @keyframes eir-pulse {
            0%, 100% { box-shadow: 0 4px 20px rgba(34, 197, 94, 0.4); }
            50% { box-shadow: 0 4px 30px rgba(34, 197, 94, 0.7); }
        }

        #eir-chat-panel {
            position: fixed;
            bottom: 92px;
            right: 24px;
            width: 420px;
            height: 600px;
            border-radius: 16px;
            overflow: hidden;
            z-index: 99998;
            box-shadow: 0 8px 40px rgba(0, 0, 0, 0.5);
            border: 1px solid rgba(88, 166, 255, 0.2);
            display: none;
            opacity: 0;
            transform: translateY(16px) scale(0.95);
            transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
        }
        #eir-chat-panel.visible {
            display: block;
            opacity: 1;
            transform: translateY(0) scale(1);
        }
        #eir-chat-panel iframe {
            width: 100%;
            height: 100%;
            border: none;
        }

        #eir-chat-badge {
            position: absolute;
            top: -2px;
            right: -2px;
            width: 14px;
            height: 14px;
            border-radius: 50%;
            background: #3fb950;
            border: 2px solid white;
        }
        #eir-chat-badge.offline { background: #f85149; }

        @media (max-width: 500px) {
            #eir-chat-panel {
                width: calc(100vw - 16px);
                height: calc(100vh - 120px);
                right: 8px;
                bottom: 84px;
                border-radius: 12px;
            }
        }
    `;
    document.head.appendChild(style);

    // ── Floating Action Button ──
    const fab = document.createElement('button');
    fab.id = 'eir-chat-fab';
    fab.innerHTML = '⚕️';
    fab.title = 'Eir Chat — Asgard AI Medical Assistant';

    const badge = document.createElement('div');
    badge.id = 'eir-chat-badge';
    fab.appendChild(badge);

    // ── Chat Panel ──
    const panel = document.createElement('div');
    panel.id = 'eir-chat-panel';

    const iframe = document.createElement('iframe');
    iframe.src = '';
    iframe.title = 'Eir Chat';
    panel.appendChild(iframe);

    document.body.appendChild(panel);
    document.body.appendChild(fab);

    // ── Toggle ──
    let isOpen = false;
    fab.addEventListener('click', () => {
        isOpen = !isOpen;
        if (isOpen) {
            if (!iframe.src || iframe.src === '' || iframe.src === window.location.href) {
                iframe.src = CHAT_URL;
            }
            panel.classList.add('visible');
            fab.classList.add('open');
            fab.innerHTML = '✕';
            fab.appendChild(badge);
        } else {
            panel.classList.remove('visible');
            fab.classList.remove('open');
            fab.innerHTML = '⚕️';
            fab.appendChild(badge);
        }
    });

    // ── Status Check ──
    async function checkBifrost() {
        try {
            const resp = await fetch(STATUS_URL);
            const data = await resp.json();
            badge.classList.toggle('offline', !data.bifrost_reachable);
        } catch {
            badge.classList.add('offline');
        }
    }
    checkBifrost();
    setInterval(checkBifrost, 30000);
})();
