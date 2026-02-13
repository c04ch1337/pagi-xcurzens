import { NexusFormData, ApiResponse } from '../types';

/**
 * Simulates the /nexus/register endpoint.
 * To test the error state, include the word "error" in the webhook URL.
 */
export const registerInfrastructure = async (data: NexusFormData): Promise<ApiResponse> => {
  return new Promise((resolve) => {
    setTimeout(() => {
      // Simulate backend validation logic
      const isSimulatedError = data.webhookUrl.toLowerCase().includes('error');

      if (isSimulatedError) {
        resolve({
          success: false,
          message: "Bandwidth Error: Please verify your Webhook URL. Registration saved, but endpoint unreachable."
        });
      } else {
        resolve({
          success: true,
          message: "Infrastructure Synchronized: Your bandwidth is now live in the XCURZENS network."
        });
      }
    }, 1500); // 1.5s artificial latency for realism
  });
};