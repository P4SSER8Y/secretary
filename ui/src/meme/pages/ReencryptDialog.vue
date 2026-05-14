<script setup lang="ts">
import { computed, getCurrentInstance, onBeforeUnmount, onMounted, ref } from 'vue';
import type { ReencryptResponse } from '../lib/struct';

const props = defineProps<{
    filter: string;
    password: string;
}>();

const emits = defineEmits<{
    quit: [];
    done: [newPassword: string];
}>();

const api = getCurrentInstance()?.appContext.config.globalProperties.$api;

const newPassword = ref('');
const confirmPassword = ref('');
const filterText = ref(props.filter);
const loading = ref(false);
const result = ref<ReencryptResponse | null>(null);

const passwordMismatch = computed(() =>
    confirmPassword.value.length > 0 && newPassword.value !== confirmPassword.value
);
const isRemoving = computed(() => newPassword.value.length === 0);
const canSubmit = computed(() =>
    !loading.value && !passwordMismatch.value
);

async function submit() {
    if (!canSubmit.value) return;
    loading.value = true;
    result.value = null;
    try {
        const res = await api?.post('reencrypt', null, {
            params: {
                new_password: newPassword.value,
                filter: filterText.value || undefined,
            },
            timeout: 120000,
        });
        result.value = res?.data;
        if (result.value && result.value.processed > 0 && result.value.errors.length === 0) {
            emits('done', newPassword.value);
        }
    } catch (e: any) {
        result.value = {
            processed: 0,
            errors: [{ uuid: '', error: e.message || 'Request failed' }],
        };
    } finally {
        loading.value = false;
    }
}

const handle_key_press = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
        emits('quit');
    }
};

onMounted(() => {
    window.addEventListener('keydown', handle_key_press);
});

onBeforeUnmount(() => {
    window.removeEventListener('keydown', handle_key_press);
});
</script>

<template>
    <div class="cover">
        <div class="card bg-base-200 shadow-xl max-w-md w-full">
            <div class="card-body">
                <h2 class="card-title">Re-encrypt Files</h2>
                <input
                    type="password"
                    placeholder="new password (leave empty to remove encryption)"
                    v-model="newPassword"
                    class="input input-bordered w-full"
                />
                <input
                    type="password"
                    placeholder="confirm password"
                    v-model="confirmPassword"
                    :class="['input input-bordered w-full', passwordMismatch ? 'input-error' : '']"
                />
                <p v-if="passwordMismatch" class="text-error text-sm">Passwords do not match</p>
                <input
                    type="text"
                    placeholder="filter (optional)"
                    v-model="filterText"
                    class="input input-bordered w-full"
                />
                <div v-if="loading" class="text-center">
                    <span class="loading loading-spinner"></span>
                    <span class="ml-2">Processing...</span>
                </div>
                <div v-if="result" class="text-sm">
                    <p class="text-success">Processed: {{ result.processed }} file(s)</p>
                    <p v-if="result.errors.length > 0" class="text-error">
                        {{ result.errors.length }} error(s):
                    </p>
                    <ul v-if="result.errors.length > 0" class="list-disc list-inside text-error text-xs">
                        <li v-for="err in result.errors" :key="err.uuid">
                            {{ err.uuid }}: {{ err.error }}
                        </li>
                    </ul>
                </div>
                <div class="card-actions justify-end mt-4">
                    <button class="btn btn-ghost btn-sm" @click="emits('quit')" :disabled="loading">
                        Cancel
                    </button>
                    <button
                        class="btn btn-sm"
                        :class="isRemoving ? 'btn-error' : 'btn-primary'"
                        @click="submit"
                        :disabled="!canSubmit"
                    >
                        {{ isRemoving ? 'Remove Encryption' : 'Re-encrypt' }}
                    </button>
                </div>
            </div>
        </div>
    </div>
</template>

<style lang="postcss" scoped>
@reference "tailwindcss";

.cover {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    right: 0;

    display: flex;
    align-items: center;
    justify-content: center;

    background-color: rgba(0, 0, 0, 0.65);
    z-index: 50;
    @apply backdrop-filter backdrop-blur-sm;
}
</style>
