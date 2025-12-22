// Auth removed - all users are treated as guests
export type UserType = "guest" | "regular";

type Entitlements = {
  maxMessagesPerDay: number;
};

export const entitlementsByUserType: Record<UserType, Entitlements> = {
  /*
   * For users without an account
   */
  guest: {
    maxMessagesPerDay: 1000,
  },

  /*
   * For users with an account
   */
  regular: {
    maxMessagesPerDay: 5000,
  },

  /*
   * TODO: For users with an account and a paid membership
   */
};
