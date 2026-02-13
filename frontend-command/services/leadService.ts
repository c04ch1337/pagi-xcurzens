
import { InfrastructureData, Lead } from '../types';

// Utility to generate mock lead data
const generateMockLeads = (count: number): Lead[] => {
  const cities = ['Dallas', 'New York', 'Los Angeles', 'Chicago', 'Miami', 'Seattle', 'Austin', 'Denver'];
  const weathers = ['Sunny 75°F', 'Cloudy 62°F', 'Rainy 55°F', 'Clear 80°F', 'Windy 45°F'];
  const intents = ['High', 'Medium', 'Low'];
  
  return Array.from({ length: count }, (_, i) => {
    const isHighIntent = Math.random() > 0.7;
    return {
      id: `LX-${1000 + i}`,
      timestamp: new Date(Date.now() - Math.random() * 10000000).toISOString().replace('T', ' ').substring(0, 19),
      query_snippet: isHighIntent 
        ? "URGENT: Requires immediate HVAC system replacement for commercial building."
        : "Looking for general pricing on solar panel maintenance.",
      city: cities[Math.floor(Math.random() * cities.length)],
      weather: weathers[Math.floor(Math.random() * weathers.length)],
      intent: isHighIntent ? 'High' : (Math.random() > 0.5 ? 'Medium' : 'Low'),
      partner_id: `P-${Math.floor(Math.random() * 50) + 10}`,
      high_intent: isHighIntent,
      highlight: isHighIntent
    };
  }).sort((a, b) => b.timestamp.localeCompare(a.timestamp));
};

/**
 * Simulates fetching data from /infrastructure/leads
 * In a real environment, this would be: 
 * const response = await fetch('/infrastructure/leads');
 * return await response.json();
 */
export const fetchInfrastructureLeads = async (): Promise<InfrastructureData> => {
  // Simulating network delay
  await new Promise(resolve => setTimeout(resolve, 300));

  const leads = generateMockLeads(15);
  const highIntentCount = leads.filter(l => l.high_intent).length;

  return {
    system_summary: {
      total_leads: 1240 + Math.floor(Math.random() * 50),
      high_intent_leads: 85 + Math.floor(Math.random() * 10),
      active_partners: 12,
      system_uptime: "99.98%"
    },
    leads: leads
  };
};
