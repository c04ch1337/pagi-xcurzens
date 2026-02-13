
export interface SystemSummary {
  total_leads: number;
  high_intent_leads: number;
  active_partners: number;
  system_uptime: string;
}

export interface Lead {
  id: string;
  timestamp: string;
  query_snippet: string;
  city: string;
  weather: string;
  intent: string;
  partner_id: string;
  high_intent: boolean;
  highlight?: boolean;
}

export interface InfrastructureData {
  system_summary: SystemSummary;
  leads: Lead[];
}

export interface AIInsight {
  status: 'normal' | 'alert' | 'critical';
  insight: string;
}
