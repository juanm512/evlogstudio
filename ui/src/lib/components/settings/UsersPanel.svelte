<script lang="ts">
  import { createQuery, useQueryClient } from '@tanstack/svelte-query';
  import { api } from '$lib/api';
  import { currentUser } from '$lib/stores';
  import type { User } from '$lib/types';
  import Modal from '$lib/components/common/Modal.svelte';
  import CustomSelect from '$lib/components/common/CustomSelect.svelte';
  import PanelHeader from '$lib/components/common/PanelHeader.svelte';
  import ConfirmModal from '$lib/components/common/ConfirmModal.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import StatusRow from '$lib/components/common/StatusRow.svelte';
  import { format } from 'date-fns';
  import { Trash2, UserPlus, Loader2, ShieldCheck, UserCog, Mail, Key, Shield } from 'lucide-svelte';

  const queryClient = useQueryClient();

  const usersQuery = createQuery(() => ({
    queryKey: ['users'],
    queryFn: () => api.get<User[]>('/api/users')
  }));

  const roleOptions = [
    { id: 'viewer', label: 'Viewer (Read Only)' },
    { id: 'admin', label: 'Admin (Full Access)' }
  ];

  let isCreateModalOpen = $state(false);
  let isRoleModalOpen = $state(false);
  let isDeleteModalOpen = $state(false);
  let selectedUser = $state<User | null>(null);
  let userToDelete = $state<User | null>(null);
  let isSubmitting = $state(false);
  let errorMsg = $state('');

  let newUser = $state({
    email: '',
    password: '',
    role: 'viewer' as 'admin' | 'viewer'
  });

  let targetRole = $state<'admin' | 'viewer'>('viewer');

  async function handleCreate() {
    isSubmitting = true;
    errorMsg = '';
    try {
      await api.post('/api/users', newUser);
      queryClient.invalidateQueries({ queryKey: ['users'] });
      isCreateModalOpen = false;
      newUser = { email: '', password: '', role: 'viewer' };
    } catch (e: any) {
      if (e.message.includes('409') || e.message.toLowerCase().includes('already exists')) {
        errorMsg = 'Email already exists';
      } else {
        errorMsg = e.message;
      }
    } finally {
      isSubmitting = false;
    }
  }

  function openRoleModal(user: User) {
    selectedUser = user;
    targetRole = user.role as 'admin' | 'viewer';
    isRoleModalOpen = true;
  }

  async function handleChangeRole() {
    if (!selectedUser) return;
    isSubmitting = true;
    try {
      await api.put(`/api/users/${selectedUser.id}/role`, { role: targetRole });
      queryClient.invalidateQueries({ queryKey: ['users'] });
      isRoleModalOpen = false;
    } catch (e: any) {
      alert(e.message);
    } finally {
      isSubmitting = false;
    }
  }

  function confirmDelete(user: User) {
    userToDelete = user;
    isDeleteModalOpen = true;
  }

  async function handleDelete() {
    if (!userToDelete) return;
    isSubmitting = true;
    try {
      await api.delete(`/api/users/${userToDelete.id}`);
      queryClient.invalidateQueries({ queryKey: ['users'] });
      isDeleteModalOpen = false;
      userToDelete = null;
    } catch (e: any) {
      alert(e.message);
    } finally {
      isSubmitting = false;
    }
  }
</script>

<div class="space-y-4">
  <PanelHeader 
    title="IAM / Users" 
    subtitle="Identity and Access Management"
    actionIcon={UserPlus}
    actionLabel="Add User"
    onAction={() => isCreateModalOpen = true}
  />

  <div class="border border-border-dim bg-surface overflow-hidden">
    <table class="w-full text-left border-collapse">
      <thead>
        <tr class="bg-surface-elevated border-b border-border-dim">
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted">Email</th>
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted text-center w-[15%]">Role</th>
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted w-[20%]">Last Login</th>
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted w-[15%]">Created</th>
          <th class="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-text-muted text-right w-[15%]">Actions</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-border-dim">
        {#if usersQuery.isLoading}
          <StatusRow colspan={5} message="Syncing directory..." />
        {:else}
          {#each usersQuery.data ?? [] as user}
            <tr class="hover:bg-surface-elevated transition-colors group">
              <td class="px-6 py-4 text-xs font-mono font-bold text-text-primary">
                <div class="flex items-center gap-2">
                  <span>{user.email}</span>
                  {#if $currentUser?.email === user.email}
                    <span class="text-[9px] bg-brand-primary/10 text-brand-primary px-1 font-sans uppercase">You</span>
                  {/if}
                </div>
              </td>
              <td class="px-6 py-4 text-center">
                <Badge variant={user.role === 'admin' ? 'info' : 'muted'}>
                  {user.role}
                </Badge>
              </td>
              <td class="px-6 py-4 text-[11px] font-mono text-text-secondary tracking-tight">
                {user.last_login ? format(new Date(user.last_login), 'yyyy/MM/dd HH:mm') : 'Never'}
              </td>
              <td class="px-6 py-4 text-[11px] font-mono text-text-muted tracking-tight">
                {format(new Date(user.created_at), 'yyyy/MM/dd')}
              </td>
              <td class="px-6 py-4 text-right">
                <div class="flex items-center justify-end gap-2">
                  <button 
                    onclick={() => openRoleModal(user)}
                    disabled={$currentUser?.email === user.email}
                    class="text-text-muted hover:text-brand-primary transition-colors p-1 disabled:opacity-20"
                    title="Change Role"
                  >
                    <UserCog size={16} />
                  </button>
                  <button 
                    onclick={() => confirmDelete(user)}
                    disabled={$currentUser?.email === user.email}
                    class="text-text-muted hover:text-brand-danger transition-colors p-1 disabled:opacity-20"
                    title="Delete User"
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>
</div>

<!-- Create User Modal -->
<Modal open={isCreateModalOpen} title="Onboard New User" onClose={() => isCreateModalOpen = false}>
  <form onsubmit={(e) => { e.preventDefault(); handleCreate(); }} class="space-y-6">
    <div class="space-y-2">
      <label for="usr-email" class="text-[10px] font-bold uppercase tracking-widest text-text-muted block">Identity (Email)</label>
      <div class="relative">
        <Mail class="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted" size={14} />
        <input 
          id="usr-email"
          type="email"
          bind:value={newUser.email}
          placeholder="user@example.com"
          required
          class="w-full bg-surface-elevated border border-border-dim p-3 pl-10 text-[13px] text-text-primary focus:border-brand-primary outline-none transition-colors"
        />
      </div>
    </div>

    <div class="space-y-2">
      <label for="usr-pw" class="text-[10px] font-bold uppercase tracking-widest text-text-muted block">Initial Password</label>
      <div class="relative">
        <Key class="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted" size={14} />
        <input 
          id="usr-pw"
          type="password"
          bind:value={newUser.password}
          placeholder="••••••••"
          required
          class="w-full bg-surface-elevated border border-border-dim p-3 pl-10 text-[13px] text-text-primary focus:border-brand-primary outline-none transition-colors"
        />
      </div>
    </div>

    <div class="space-y-2">
      <CustomSelect 
        label="Access Level"
        options={roleOptions}
        value={newUser.role}
        onSelect={(id) => newUser.role = id as 'admin' | 'viewer'}
      />
    </div>

    {#if errorMsg}
      <div class="text-brand-danger text-[10px] font-bold uppercase bg-brand-danger/5 p-3 border-l-2 border-brand-danger font-mono">
        Directory_Error: {errorMsg}
      </div>
    {/if}

    <div class="pt-4">
      <button 
        type="submit"
        disabled={isSubmitting}
        class="w-full bg-brand-primary text-white text-[11px] font-bold uppercase tracking-[2px] py-3.5 hover:bg-blue-600 transition-all disabled:opacity-50 flex items-center justify-center gap-2"
      >
        {#if isSubmitting}
          <Loader2 size={16} class="animate-spin" />
          Creating Identity...
        {:else}
          Create User
        {/if}
      </button>
    </div>
  </form>
</Modal>

<!-- Change Role Modal -->
<Modal open={isRoleModalOpen} title="Update Permissions" onClose={() => isRoleModalOpen = false}>
  <div class="space-y-6">
    <div class="space-y-2">
      <p class="text-[11px] text-text-secondary">
        Changing role for <span class="font-mono font-bold text-text-primary">{selectedUser?.email}</span>
      </p>
      <CustomSelect 
        label="New Role"
        options={roleOptions}
        value={targetRole}
        onSelect={(id) => targetRole = id as 'admin' | 'viewer'}
      />
    </div>

    <div class="pt-4">
      <button 
        onclick={handleChangeRole}
        disabled={isSubmitting}
        class="w-full bg-brand-primary text-white text-[11px] font-bold uppercase tracking-[2px] py-3.5 hover:bg-blue-600 transition-all disabled:opacity-50 flex items-center justify-center gap-2"
      >
        {#if isSubmitting}
          <Loader2 size={16} class="animate-spin" />
          Updating...
        {:else}
          Save Changes
        {/if}
      </button>
    </div>
  </div>
</Modal>

<!-- Delete Confirmation Modal -->
<ConfirmModal 
  open={isDeleteModalOpen}
  title="Confirm Identity Deletion"
  description="Are you sure you want to delete user {userToDelete?.email}?"
  impactLabel="Critical Warning"
  impactDescription="This user will lose all access to the studio. This action cannot be undone."
  confirmLabel="Revoke All Access"
  {isSubmitting}
  onConfirm={handleDelete}
  onCancel={() => isDeleteModalOpen = false}
/>

<style>
  input, button {
    border-radius: 0 !important;
  }
</style>
