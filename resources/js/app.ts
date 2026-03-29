import '../css/app.css';

import { createInertiaApp } from '@inertiajs/vue3';

createInertiaApp({
    title: (title) => (title ? `${title} | Telescope` : 'Telescope'),
    progress: {
        color: '#00d4ff',
    },
});
