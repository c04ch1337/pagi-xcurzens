
export interface GeoContext {
  city: string;
  weather: string;
}

export enum MessageRole {
  USER = 'user',
  SCOUT = 'scout'
}

export interface Message {
  id: string;
  role: MessageRole;
  text: string;
  timestamp: number;
}

export interface ScoutRequest {
  query: string;
  city: string;
  weather: string;
  businessName?: string;
}
