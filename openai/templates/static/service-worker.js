self.addEventListener('install', event => {
    event.waitUntil(
        caches.open('pandora-cloud-cache').then(function (cache) {
            return cache.addAll([
                '/apple-touch-icon.png',
                '/favicon-16x16.png',
                '/favicon-32x32.png',
                '/ulp/react-components/1.66.5/css/main.cdn.min.css',
                '/sweetalert2/bulma.min.css',
                '/sweetalert2/sweetalert2.all.min-bc15590d.js',
                '/fonts/colfax/ColfaxAIRegular.woff2',
                '/fonts/colfax/ColfaxAIRegular.woff',
                '/fonts/colfax/ColfaxAIRegularItalic.woff2',
                '/fonts/colfax/ColfaxAIRegularItalic.woff',
                '/fonts/colfax/ColfaxAIBold.woff2',
                '/fonts/colfax/ColfaxAIBold.woff',
                '/fonts/colfax/ColfaxAIBoldItalic.woff2',
                '/fonts/colfax/ColfaxAIBoldItalic.woff',
                '/fonts/soehne/soehne-buch-kursiv.woff2',
                '/fonts/soehne/soehne-buch.woff2',
                '/fonts/soehne/soehne-halbfett-kursiv.woff2',
                '/fonts/soehne/soehne-halbfett.woff2',
                '/fonts/soehne/soehne-kraftig-kursiv.woff2',
                '/fonts/soehne/soehne-kraftig.woff2',
                '/fonts/soehne/soehne-mono-buch-kursiv.woff2',
                '/fonts/soehne/soehne-mono-buch.woff2',
                '/fonts/soehne/soehne-mono-halbfett.woff2',
                '/_next/static/chunks/012ff928-bcfa62e3ac82441c.js',
                '/_next/static/chunks/1f110208-cda4026aba1898fb.js',
                '/_next/static/chunks/259-c6320349d8f3ff4a.js',
                '/_next/static/chunks/58-87db5ef127e7d0b9.js',
                '/_next/static/chunks/686-7a99a2d97a992914.js',
                '/_next/static/chunks/68a27ff6-a453fd719d5bf767.js',
                '/_next/static/chunks/734-99309a157861fd84.js',
                '/_next/static/chunks/bd26816a-981e1ddc27b37cc6.js',
                '/_next/static/chunks/framework-e23f030857e925d4.js',
                '/_next/static/chunks/main-2f10fecca4c74462.js',
                '/_next/static/chunks/pages/app-878ac43f93e00b3f.js',
                '/_next/static/chunks/pages/error-433a1bbdb23dd341.js',
                '/_next/static/chunks/pages/account/cancel-63cd9f049103272b.js',
                '/_next/static/chunks/pages/account/manage-6ac6d4f0510ced68.js',
                '/_next/static/chunks/pages/account/upgrade-d6b322741680e2b4.js',
                '/_next/static/chunks/pages/aip/pluginId/oauth/callback-389963a554a230d2.js',
                '/_next/static/chunks/pages/auth/error-c7951a77c5f4547f.js',
                '/_next/static/chunks/pages/auth/ext_callback-927659025ea31258.js',
                '/_next/static/chunks/pages/auth/ext_callback_refresh-478ebccc4055d75b.js',
                '/_next/static/chunks/pages/auth/login-f4fdb51b436aaaf4.js',
                '/_next/static/chunks/pages/auth/logout-47cc26eb7b585e67.js',
                '/_next/static/chunks/pages/auth/mocked_login-d5fbb97bc5d39e59.js',
                '/_next/static/chunks/pages/bypass-338530f42d5b2105.js',
                '/_next/static/chunks/pages/c/chatId-c0db665988dd1d67.js',
                '/_next/static/chunks/pages/index-1401d2c2b63d99cf.js',
                '/_next/static/chunks/pages/payments/business-e449df976df219cb.js',
                '/_next/static/chunks/pages/payments/success-66b11e86067b001d.js',
                '/_next/static/chunks/pages/share/shareParams-875a033295abd238.js',
                '/_next/static/chunks/pages/status-6557d60655b68492.js',
                '/_next/static/chunks/polyfills-c67a75d1b6f99dc8.js',
                '/_next/static/chunks/webpack-c08a82b5c21eeb39.js',
                '/_next/static/css/df35a37d1f08004f.css',
                '/_next/static/cx416mT2Lb0ZTj5FxFg1l/buildManifest1.js',
                '/_next/static/cx416mT2Lb0ZTj5FxFg1l/ssgManifest.js',
            ]);
        })
    );
});

self.addEventListener('fetch', function (event) {
    event.respondWith(
        caches.match(event.request)
            .then(function (response) {
                    if (response) {
                        return response;
                    }
                    return fetch(event.request);
                }
            )
    );
});
