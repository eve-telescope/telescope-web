/** The signed-in user of the website (the user model's JSON). */
export type User = {
  id: number;
  character_id: number;
  character_name: string;
  corporation_id: number | null;
  alliance_id: number | null;
  last_active_at: string | null;
  created_at: string;
  updated_at: string;
};

/** The props every page gets. */
export type SharedProps = {
  name: string;
  auth: { user: User | null };
  /** The latest release of the desktop app, `null` when GitHub failed. */
  appVersion: string | null;
};

export type HomeProps = SharedProps & {
  /** The local demo login is on (never in production). */
  demoLoginEnabled: boolean;
};

export type Share = {
  code: string;
  pilots: string[];
  pilotCount: number;
  views: number;
  createdAt: string;
};

export type ShareProps = SharedProps & { share: Share };

export type AuthTokenProps = SharedProps & {
  token: string;
  deepLink: string;
  characterName: string;
};
