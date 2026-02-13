import React, { useState } from 'react';
import { NexusFormData, ServiceType, RegistrationStatus } from '../types';
import { registerInfrastructure } from '../services/nexusService';
import { StatusPanel } from './StatusPanel';
import { LoaderIcon, AlertTriangleIcon, ShareIcon, MailIcon, TwitterIcon, LinkedInIcon, LayoutIcon } from './Icons';

interface NexusFormProps {
  onGoToDashboard?: () => void;
}

export const NexusForm: React.FC<NexusFormProps> = ({ onGoToDashboard }) => {
  const [formData, setFormData] = useState<NexusFormData>({
    businessName: '',
    primaryCity: '',
    serviceType: ServiceType.UNSELECTED,
    webhookUrl: '',
    contactEmail: ''
  });

  const [errors, setErrors] = useState<Partial<Record<keyof NexusFormData, string>>>({});
  const [status, setStatus] = useState<RegistrationStatus>('idle');
  const [responseMessage, setResponseMessage] = useState<string>('');
  const [showConfirm, setShowConfirm] = useState<boolean>(false);
  const [showShareModal, setShowShareModal] = useState<boolean>(false);

  const validateForm = (): boolean => {
    const newErrors: Partial<Record<keyof NexusFormData, string>> = {};
    let isValid = true;

    if (!formData.businessName.trim() || formData.businessName.length < 2) {
      newErrors.businessName = 'Business name must be at least 2 characters.';
      isValid = false;
    }

    if (!formData.primaryCity.trim() || formData.primaryCity.length < 2) {
      newErrors.primaryCity = 'City name must be at least 2 characters.';
      isValid = false;
    }

    if (!formData.serviceType) {
      newErrors.serviceType = 'Please select a valid service type.';
      isValid = false;
    }

    // URL Validation
    try {
      const url = new URL(formData.webhookUrl);
      if (!['http:', 'https:'].includes(url.protocol)) {
         newErrors.webhookUrl = 'URL must start with http:// or https://';
         isValid = false;
      }
    } catch (e) {
      if (!formData.webhookUrl) {
         newErrors.webhookUrl = 'Webhook URL is required.';
      } else {
         newErrors.webhookUrl = 'Please enter a valid URL (e.g., https://api.example.com).';
      }
      isValid = false;
    }

    // Email Validation
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!formData.contactEmail || !emailRegex.test(formData.contactEmail)) {
      newErrors.contactEmail = 'Please enter a valid email address.';
      isValid = false;
    }

    setErrors(newErrors);
    return isValid;
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target;
    setFormData(prev => ({ ...prev, [name]: value }));
    
    // Clear error for field on change
    if (errors[name as keyof NexusFormData]) {
      setErrors(prev => ({ ...prev, [name]: undefined }));
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (status === 'submitting') return;

    if (validateForm()) {
        setShowConfirm(true);
    }
  };

  const handleConfirmRegistration = async () => {
    setShowConfirm(false);
    setStatus('submitting');
    setResponseMessage('');

    try {
      const response = await registerInfrastructure(formData);
      setResponseMessage(response.message);
      setStatus(response.success ? 'success' : 'error');
    } catch (error) {
      setResponseMessage("Critical Error: Unable to reach Nexus core.");
      setStatus('error');
    }
  };

  const getInputClass = (fieldName: keyof NexusFormData) => {
    const baseClass = "w-full bg-nexus-navy/50 border rounded-lg px-4 py-3 text-white placeholder-gray-500 focus:outline-none transition-all";
    if (errors[fieldName]) {
      return `${baseClass} border-red-500 focus:border-red-500 focus:ring-1 focus:ring-red-500`;
    }
    return `${baseClass} border-white/20 focus:border-nexus-orange focus:ring-1 focus:ring-nexus-orange`;
  };

  return (
    <div className="w-full max-w-2xl mx-auto">
      {/* Glassmorphism Card */}
      <div className="bg-nexus-glass backdrop-blur-xl border border-nexus-glassBorder rounded-2xl p-8 shadow-2xl relative overflow-hidden">
        
        {/* Decorative Top Accent */}
        <div className="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-transparent via-nexus-orange to-transparent opacity-50"></div>

        <div className="mb-8 text-center sm:text-left">
          <h2 className="text-2xl font-semibold text-white tracking-tight">
            Partner Infrastructure Registration
          </h2>
          <p className="text-gray-400 text-sm mt-2">
            Initialize your node in the XCURZENS network.
          </p>
        </div>

        <form onSubmit={handleSubmit} className="space-y-6" noValidate>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            
            {/* Business Name */}
            <div className="space-y-2">
              <label htmlFor="businessName" className="block text-xs font-medium text-gray-300 uppercase tracking-wider">
                Business Name
              </label>
              <input
                type="text"
                id="businessName"
                name="businessName"
                value={formData.businessName}
                onChange={handleChange}
                placeholder="e.g. Coastal Charters LLC"
                className={getInputClass('businessName')}
              />
              {errors.businessName && (
                <p className="text-red-400 text-xs flex items-center gap-1 animate-fade-in-up">
                  <AlertTriangleIcon className="w-3 h-3" /> {errors.businessName}
                </p>
              )}
            </div>

            {/* Primary City */}
            <div className="space-y-2">
              <label htmlFor="primaryCity" className="block text-xs font-medium text-gray-300 uppercase tracking-wider">
                Primary City
              </label>
              <input
                type="text"
                id="primaryCity"
                name="primaryCity"
                value={formData.primaryCity}
                onChange={handleChange}
                placeholder="e.g. Corpus Christi"
                className={getInputClass('primaryCity')}
              />
              {errors.primaryCity && (
                <p className="text-red-400 text-xs flex items-center gap-1 animate-fade-in-up">
                  <AlertTriangleIcon className="w-3 h-3" /> {errors.primaryCity}
                </p>
              )}
            </div>

            {/* Service Type */}
            <div className="space-y-2 md:col-span-2">
              <label htmlFor="serviceType" className="block text-xs font-medium text-gray-300 uppercase tracking-wider">
                Service Type
              </label>
              <div className="relative">
                <select
                  id="serviceType"
                  name="serviceType"
                  value={formData.serviceType}
                  onChange={handleChange}
                  className={`${getInputClass('serviceType')} appearance-none`}
                >
                  <option value="" disabled className="text-gray-500">Select Service Vector</option>
                  <option value={ServiceType.CHARTER}>Charter</option>
                  <option value={ServiceType.BEACH_BOX}>Beach Box</option>
                  <option value={ServiceType.EQUIPMENT_RENTAL}>Equipment Rental</option>
                  <option value={ServiceType.LOGISTICS}>Logistics</option>
                </select>
                <div className="absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none">
                  <svg className="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M19 9l-7 7-7-7"></path></svg>
                </div>
              </div>
              {errors.serviceType && (
                <p className="text-red-400 text-xs flex items-center gap-1 animate-fade-in-up">
                  <AlertTriangleIcon className="w-3 h-3" /> {errors.serviceType}
                </p>
              )}
            </div>

            {/* Webhook URL */}
            <div className="space-y-2 md:col-span-2">
              <label htmlFor="webhookUrl" className="block text-xs font-medium text-gray-300 uppercase tracking-wider flex justify-between">
                <span>Webhook / API URL</span>
                <span className="text-[10px] text-nexus-orange opacity-80 normal-case">For Lead Delivery</span>
              </label>
              <div className="relative">
                 <div className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-500">
                    <svg className={`w-4 h-4 ${errors.webhookUrl ? 'text-red-400' : ''}`} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"></path></svg>
                 </div>
                <input
                  type="url"
                  id="webhookUrl"
                  name="webhookUrl"
                  value={formData.webhookUrl}
                  onChange={handleChange}
                  placeholder="https://api.your-infrastructure.com/v1/leads"
                  className={`${getInputClass('webhookUrl')} pl-10 pr-4 font-mono text-sm`}
                />
              </div>
              {errors.webhookUrl && (
                <p className="text-red-400 text-xs flex items-center gap-1 animate-fade-in-up">
                  <AlertTriangleIcon className="w-3 h-3" /> {errors.webhookUrl}
                </p>
              )}
            </div>

            {/* Contact Email */}
            <div className="space-y-2 md:col-span-2">
              <label htmlFor="contactEmail" className="block text-xs font-medium text-gray-300 uppercase tracking-wider">
                Contact Email
              </label>
              <input
                type="email"
                id="contactEmail"
                name="contactEmail"
                value={formData.contactEmail}
                onChange={handleChange}
                placeholder="captain@example.com"
                className={getInputClass('contactEmail')}
              />
              {errors.contactEmail && (
                <p className="text-red-400 text-xs flex items-center gap-1 animate-fade-in-up">
                  <AlertTriangleIcon className="w-3 h-3" /> {errors.contactEmail}
                </p>
              )}
            </div>
          </div>

          <div className="pt-4">
            <button
              type="submit"
              disabled={status === 'submitting'}
              className="w-full bg-nexus-orange hover:bg-orange-500 text-white font-semibold py-4 rounded-lg shadow-lg hover:shadow-orange-500/20 transition-all duration-300 flex items-center justify-center gap-2 disabled:opacity-70 disabled:cursor-not-allowed transform active:scale-[0.99]"
            >
              {status === 'submitting' ? (
                <>
                  <LoaderIcon className="w-5 h-5 animate-spin" />
                  <span>Synchronizing...</span>
                </>
              ) : (
                <span>Register Bandwidth</span>
              )}
            </button>
          </div>
        </form>

        {/* Dynamic Status Panel */}
        <StatusPanel status={status} message={responseMessage} />

        {/* Success Actions */}
        {status === 'success' && (
           <div className="mt-6 flex flex-col sm:flex-row justify-center gap-4 animate-fade-in-up delay-200">
              <button
                onClick={() => setShowShareModal(true)}
                type="button"
                className="flex items-center justify-center gap-2 px-6 py-2.5 text-sm font-medium text-nexus-orange border border-nexus-orange/30 rounded-lg hover:bg-nexus-orange/10 transition-colors"
              >
                <ShareIcon className="w-4 h-4" />
                Share Registration
              </button>
              
              {onGoToDashboard && (
                <button
                  onClick={onGoToDashboard}
                  type="button"
                  className="flex items-center justify-center gap-2 px-6 py-2.5 text-sm font-medium bg-white/10 text-white rounded-lg hover:bg-white/20 transition-colors shadow-lg"
                >
                  <LayoutIcon className="w-4 h-4" />
                  Enter Terminal Dashboard
                </button>
              )}
           </div>
        )}

        {/* Confirmation Dialog Overlay */}
        {showConfirm && (
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm animate-fade-in">
            <div className="bg-[#0A2569] border border-white/10 rounded-xl p-6 shadow-2xl max-w-sm w-full transform transition-all scale-100">
              <h3 className="text-xl font-semibold text-white mb-3">Confirm Registration</h3>
              <p className="text-gray-300 text-sm mb-6 leading-relaxed">
                Are you sure you want to register this infrastructure?
              </p>
              <div className="flex gap-3 justify-end">
                <button
                  type="button"
                  onClick={() => setShowConfirm(false)}
                  className="px-4 py-2 text-sm font-medium text-gray-400 hover:text-white transition-colors"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={handleConfirmRegistration}
                  className="px-5 py-2 text-sm font-medium bg-nexus-orange text-white rounded-lg hover:bg-orange-500 transition-colors shadow-lg shadow-nexus-orange/20"
                >
                  Confirm
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Share Dialog Overlay */}
        {showShareModal && (
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm animate-fade-in">
             <div className="bg-[#0A2569] border border-white/10 rounded-xl p-6 shadow-2xl max-w-sm w-full transform transition-all scale-100 relative">
               
               <button 
                 onClick={() => setShowShareModal(false)}
                 className="absolute top-4 right-4 text-gray-400 hover:text-white"
               >
                 <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M6 18L18 6M6 6l12 12" /></svg>
               </button>

               <h3 className="text-xl font-semibold text-white mb-2 flex items-center gap-2">
                 <ShareIcon className="w-5 h-5 text-nexus-orange" />
                 Share Activation
               </h3>
               <p className="text-gray-300 text-sm mb-6 leading-relaxed">
                 Notify your network that your infrastructure is now live on XCURZENS.
               </p>
               
               <div className="flex flex-col gap-3">
                  <a href="mailto:?subject=XCURZENS Infrastructure Live&body=Our bandwidth is now synchronized with the XCURZENS network." className="flex items-center gap-3 px-4 py-3 bg-white/5 hover:bg-white/10 rounded-lg transition-colors border border-white/5 hover:border-nexus-orange/30 group">
                    <MailIcon className="w-5 h-5 text-gray-300 group-hover:text-nexus-orange" />
                    <span className="text-sm font-medium">Email</span>
                  </a>
                  <a href="https://twitter.com/intent/tweet?text=Just registered our infrastructure on XCURZENS Nexus." target="_blank" rel="noopener noreferrer" className="flex items-center gap-3 px-4 py-3 bg-white/5 hover:bg-white/10 rounded-lg transition-colors border border-white/5 hover:border-nexus-orange/30 group">
                    <TwitterIcon className="w-5 h-5 text-gray-300 group-hover:text-blue-400" />
                    <span className="text-sm font-medium">Twitter</span>
                  </a>
                  <a href="https://www.linkedin.com/sharing/share-offsite/?url=https://xcurzens.com" target="_blank" rel="noopener noreferrer" className="flex items-center gap-3 px-4 py-3 bg-white/5 hover:bg-white/10 rounded-lg transition-colors border border-white/5 hover:border-nexus-orange/30 group">
                    <LinkedInIcon className="w-5 h-5 text-gray-300 group-hover:text-blue-600" />
                    <span className="text-sm font-medium">LinkedIn</span>
                  </a>
               </div>
             </div>
          </div>
        )}

      </div>
    </div>
  );
};