# Telescope Web

<p align="center">
  <img src="public/favicon.svg" width="128" height="128" alt="Telescope Logo">
</p>

<p align="center">
  <strong>Web Interface for Telescope</strong><br>
  Landing page & share link handler for the EVE Online intel tool
</p>

<p align="center">
  <a href="https://github.com/eve-telescope/telescope-web/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/eve-telescope/telescope-web?style=flat-square" alt="License">
  </a>
</p>

---

## Overview

Telescope Web serves two purposes:

1. **Landing Page** — Download links and feature overview for the Telescope app
2. **Share Handler** — Handles shared scan links and redirects to the desktop app

When someone shares a scan link like `https://telescope.example.com/s/ABC123`, the web app:
- Displays the shared pilot list
- Attempts to open the Telescope desktop app via deep link (`telescope://s/ABC123`)
- Provides fallback options if the app isn't installed

## Features

- 🌐 **Landing Page** — EVE-themed download page with feature highlights
- 🔗 **Share Links** — Short URLs for sharing scans (`/s/{code}`)
- 📋 **Copy Fallback** — Users without the app can copy pilot names
- 🌙 **Dark Theme** — EVE Online inspired aesthetic
- ⚡ **Fast** — Built with Laravel + Inertia + Vue

## Tech Stack

- **Backend**: Laravel 12, PHP 8.4
- **Frontend**: Vue 3, Inertia.js, TypeScript
- **Styling**: Tailwind CSS 4
- **Database**: SQLite (or any Laravel-supported DB)

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/share` | Create a new share |
| `GET` | `/api/share/{code}` | Get share data (JSON) |
| `GET` | `/s/{code}` | View share page (HTML) |

### Create Share

```bash
curl -X POST https://telescope.example.com/api/share \
  -H "Content-Type: application/json" \
  -d '{"pilots": ["Pilot One", "Pilot Two", "Pilot Three"]}'
```

Response:
```json
{
  "code": "ABC123",
  "url": "https://telescope.example.com/s/ABC123"
}
```

## Development

### Prerequisites

- PHP 8.4+
- Composer
- Node.js 18+
- SQLite (or MySQL/PostgreSQL)

### Setup

```bash
# Clone the repository
git clone https://github.com/eve-telescope/telescope-web.git
cd telescope-web

# Install PHP dependencies
composer install

# Install Node dependencies
npm install

# Copy environment file
cp .env.example .env

# Generate app key
php artisan key:generate

# Run migrations
php artisan migrate

# Start development server
composer run dev
```

### Project Structure

```
telescope-web/
├── app/
│   ├── Http/Controllers/
│   │   └── ShareController.php    # Share API & page controller
│   └── Models/
│       └── Share.php              # Share model
├── resources/
│   ├── js/
│   │   └── pages/
│   │       ├── Welcome.vue        # Landing page
│   │       └── Share.vue          # Share view page
│   └── css/
│       └── app.css                # Tailwind + EVE theme
├── routes/
│   ├── api.php                    # API routes
│   └── web.php                    # Web routes
└── database/
    └── migrations/                # Database migrations
```

## Deployment

### Environment Variables

```env
APP_URL=https://telescope.example.com
```

### Production Build

```bash
npm run build
php artisan config:cache
php artisan route:cache
php artisan view:cache
```

## Related

- [telescope-app](https://github.com/eve-telescope/telescope-app) — Desktop application (Tauri + Vue)

## License

MIT © [eve-telescope](https://github.com/eve-telescope)

---

<p align="center">
  <sub>Not affiliated with CCP Games. EVE Online and all related logos are trademarks of CCP hf.</sub>
</p>


