/**
 * Unified WhatsApp Inbox State (Doc 16 §6)
 * Real-time SSE support, virtualised conversation handling, AI drafts with 3 actions,
 * keyboard navigation, and Rx-linked bulk action exclusion (Invariant I-6).
 */

export interface MessageItem {
  id: string;
  sender: "CUSTOMER" | "AGENT" | "AI_DRAFT";
  text?: string;
  mediaUrl?: string;
  mediaType?: "IMAGE" | "AUDIO" | "DOCUMENT";
  audioTranscript?: string;
  aiConfidence?: number;
  originalAiDraft?: string;
  timestamp: string;
}

export interface ConversationItem {
  id: string;
  customerId: string;
  customerName: string;
  phone: string;
  branchId: string;
  lastMessage: string;
  hasPrescription: boolean;
  unreadCount: number;
  status: "ACTIVE" | "PENDING_PHARMACIST" | "CLOSED";
  messages: MessageItem[];
}

export class InboxManager {
  public conversations: ConversationItem[] = [];
  public selectedConversationId: string | null = null;
  public sseConnected: boolean = false;
  public reconnecting: boolean = false;

  constructor(conversations: ConversationItem[] = []) {
    this.conversations = conversations;
    if (conversations.length > 0) {
      this.selectedConversationId = conversations[0].id;
    }
  }

  public get selectedConversation(): ConversationItem | undefined {
    return this.conversations.find((c) => c.id === this.selectedConversationId);
  }

  // Keyboard navigation: j (down), k (up)
  public navigateNext(): void {
    if (this.conversations.length === 0) return;
    const currentIndex = this.conversations.findIndex((c) => c.id === this.selectedConversationId);
    if (currentIndex < this.conversations.length - 1) {
      this.selectedConversationId = this.conversations[currentIndex + 1].id;
    }
  }

  public navigatePrevious(): void {
    if (this.conversations.length === 0) return;
    const currentIndex = this.conversations.findIndex((c) => c.id === this.selectedConversationId);
    if (currentIndex > 0) {
      this.selectedConversationId = this.conversations[currentIndex - 1].id;
    }
  }

  // AI Draft 3 Actions: Send, Edit, Discard (Doc 16 §6)
  public sendAiDraft(conversationId: string, messageId: string): void {
    const conv = this.conversations.find((c) => c.id === conversationId);
    if (!conv) return;
    const draft = conv.messages.find((m) => m.id === messageId && m.sender === "AI_DRAFT");
    if (draft) {
      draft.sender = "AGENT"; // Sent!
    }
  }

  public editAiDraft(conversationId: string, messageId: string, editedText: string): void {
    const conv = this.conversations.find((c) => c.id === conversationId);
    if (!conv) return;
    const draft = conv.messages.find((m) => m.id === messageId);
    if (draft) {
      if (!draft.originalAiDraft) {
        draft.originalAiDraft = draft.text; // Preserve original for AI training loop (Doc 16 §6)
      }
      draft.text = editedText;
    }
  }

  public discardAiDraft(conversationId: string, messageId: string): void {
    const conv = this.conversations.find((c) => c.id === conversationId);
    if (!conv) return;
    conv.messages = conv.messages.filter((m) => m.id !== messageId);
  }

  // Invariant I-6: Rx-linked conversations MUST NEVER be included in bulk sends
  public filterEligibleForBulkSend(targetIds: string[]): string[] {
    return targetIds.filter((id) => {
      const conv = this.conversations.find((c) => c.id === id);
      return conv && !conv.hasPrescription;
    });
  }

  // SSE Reconnection & Replay handler
  public handleSseConnectionDrop(): void {
    this.sseConnected = false;
    this.reconnecting = true;
  }

  public handleSseReconnected(missedMessages: { convId: string; message: MessageItem }[]): void {
    this.sseConnected = true;
    this.reconnecting = false;
    for (const item of missedMessages) {
      const conv = this.conversations.find((c) => c.id === item.convId);
      if (conv && !conv.messages.some((m) => m.id === item.message.id)) {
        conv.messages.push(item.message);
      }
    }
  }
}
