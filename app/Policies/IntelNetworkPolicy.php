<?php

namespace App\Policies;

use App\Enums\Permission;
use App\Models\IntelNetwork;
use App\Models\User;

class IntelNetworkPolicy
{
    public function view(User $user, IntelNetwork $network): bool
    {
        $permission = $network->getUserPermission($user);

        return $permission instanceof Permission;
    }

    public function addEntry(User $user, IntelNetwork $network): bool
    {
        $permission = $network->getUserPermission($user);

        return $permission instanceof Permission && $permission->isAtLeast(Permission::Member);
    }

    public function update(User $user, IntelNetwork $network): bool
    {
        $permission = $network->getUserPermission($user);

        return $permission instanceof Permission && $permission->isAtLeast(Permission::Manager);
    }

    public function manageAccess(User $user, IntelNetwork $network): bool
    {
        $permission = $network->getUserPermission($user);

        return $permission instanceof Permission && $permission->isAtLeast(Permission::Manager);
    }

    public function delete(User $user, IntelNetwork $network): bool
    {
        return $network->accesses()
            ->where('accessible_id', $user->character_id)
            ->where('is_owner', true)
            ->exists();
    }
}
