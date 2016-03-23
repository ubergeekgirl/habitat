// Copyright:: Copyright (c) 2016 Chef Software, Inc.
//
// The terms of the Evaluation Agreement (Bldr) between Chef Software Inc. and the party accessing
// this file ("Licensee") apply to Licensee's use of the Software until such time that the Software
// is made available under an open source license such as the Apache 2.0 License.

import {AppStore} from "../AppStore";
import {Component} from "angular2/core";
import {GravatarComponent} from "../GravatarComponent";
import {RouterLink} from "angular2/router";

@Component({
    directives: [GravatarComponent, RouterLink],
    template: `
    <div class="hab-organizations">
        <h2>Organizations</h2>
        <hr>
        <div *ngIf="orgs.size === 0">
            <h3 class="hero">
                You don't currently have any organizations, let's add one now.
            </h3>
            <form>
                <p>
                    Create an organization, then start adding projects and users.
                </p>
                <p>
                    <a class="button" [routerLink]='["OrganizationCreate"]'>
                        Add Organization
                    </a>
                </p>
            </form>
            <div class="info">
                <p>Organizations allow you to do the following:</p>
                <ul>
                    <li>
                        Invite users to manage a project
                    </li>
                    <li>
                        List public projects under your organization profile
                    </li>
                    <li>
                        Set admin permissions for users to edit organization
                        settings
                    </li>
                </ul>
            </div>
        </div>
        <div *ngIf="orgs.size > 0">
            <ul>
                <li *ngFor="#org of orgs">
                    <a href="#" class="hab-item-list">
                        <gravatar size=32 email="{{org.email}}"></gravatar>
                        {{org.name}}
                        <span class="count">
                            <img src="/node_modules/octicons/svg/organization.svg">
                            {{org.members.size}}
                        </span>
                    </a>
                </li>
            </ul>
        </div>
    </div>`
})

export class OrganizationsPageComponent {
    constructor(private store: AppStore) {}

    get orgs() { return this.store.getState().orgs.all; }
}