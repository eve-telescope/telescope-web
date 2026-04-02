<?php

use App\Models\IntelNetwork;
use App\Models\User;
use Illuminate\Support\Facades\Broadcast;

Broadcast::channel('intel-network.{networkId}', function (User $user, int $networkId) {
    $network = IntelNetwork::find($networkId);

    if (! $network) {
        return false;
    }

    return $network->getUserPermission($user) !== null;
});
