export interface NexusFormData {
  businessName: string;
  primaryCity: string;
  serviceType: ServiceType;
  webhookUrl: string;
  contactEmail: string;
}

export enum ServiceType {
  CHARTER = 'Charter',
  BEACH_BOX = 'Beach Box',
  EQUIPMENT_RENTAL = 'Equipment Rental',
  LOGISTICS = 'Logistics',
  UNSELECTED = ''
}

export type RegistrationStatus = 'idle' | 'submitting' | 'success' | 'error';

export interface ApiResponse {
  success: boolean;
  message: string;
}

export interface Lead {
  id: string;
  source: string;
  sector: string;
  status: 'new' | 'processing' | 'synced';
  timestamp: string;
}

export interface SystemLog {
  id: number;
  message: string;
  type: 'info' | 'success' | 'warning';
  timestamp: string;
}